//! THE SEAM between the audio engine and the browser — what
//! `Graphics::new_headless` is to drawing. The engine names every Web Audio
//! type through this module, never through `web_sys` directly:
//!
//! - on wasm32 these ARE the `web_sys` types (plain re-exports: the shipped
//!   game compiles to the very same code — the release `.wasm` is
//!   byte-identical to the one built before this module existed);
//! - under `cargo test` they are the RECORDING MOCK below: the same names and
//!   the subset of methods the engine calls, each one appending to a
//!   [`Graph`] instead of touching a browser. `AudioEngine::new()` then runs
//!   natively, and a test can play any sound and read back the node graph it
//!   built (`src/audio/engine/tests.rs`).
//!
//! A new `web_sys` call in the engine = the same method on the mock; the
//! native build fails until it exists.

#[cfg(target_arch = "wasm32")]
pub use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioDestinationNode, AudioNode, AudioParam,
    AudioScheduledSourceNode, BaseAudioContext, BiquadFilterNode, BiquadFilterType, GainNode,
    OfflineAudioContext, OscillatorType, OverSampleType,
};

#[cfg(not(target_arch = "wasm32"))]
pub use mock::*;

#[cfg(not(target_arch = "wasm32"))]
mod mock {
    //! The recording mock. One [`Graph`] per context (live or offline); every
    //! context registers its graph in a thread-local list so a test can reach
    //! the offline graphs the bake builds internally ([`graphs`]).
    use std::cell::RefCell;
    use std::marker::PhantomData;
    use std::ops::Deref;
    use std::rc::Rc;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NodeKind {
        Destination,
        Gain,
        Oscillator,
        Biquad,
        BufferSource,
        Convolver,
        Compressor,
        WaveShaper,
        Delay,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum EventKind {
        Set,
        LinearRamp,
        ExpRamp,
        /// A value curve; `value` is its peak |v|, `end` = start + duration.
        Curve,
        Cancel,
    }

    /// One automation call on an `AudioParam`.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ParamEvent {
        pub node: usize,
        pub param: &'static str,
        pub kind: EventKind,
        pub value: f32,
        pub time: f64,
        pub end: f64,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct NodeRec {
        pub kind: NodeKind,
        /// Oscillator / biquad type, as its `Debug` name (`"Sawtooth"`, …).
        pub type_name: Option<String>,
        /// Frames of the buffer a source / convolver was given.
        pub buffer_frames: Option<u32>,
        pub looping: bool,
        /// `start(when, offset)` / `stop(when)` of a scheduled source.
        pub start: Option<(f64, f64)>,
        pub stop: Option<f64>,
    }

    /// Everything one context was asked to build.
    #[derive(Debug, Default)]
    pub struct Graph {
        pub sample_rate: f32,
        /// `Some(frames)` for an `OfflineAudioContext`.
        pub offline_frames: Option<u32>,
        /// What `current_time()` answers (a test may move it).
        pub now: f64,
        pub nodes: Vec<NodeRec>,
        /// `(from, to)` node indices, in connection order.
        pub edges: Vec<(usize, usize)>,
        pub events: Vec<ParamEvent>,
        /// `(from node, to node, its param)`: a node driving an `AudioParam`.
        pub mod_edges: Vec<(usize, usize, &'static str)>,
    }

    impl Graph {
        pub fn count(&self, kind: NodeKind) -> usize {
            self.nodes.iter().filter(|n| n.kind == kind).count()
        }
        /// Is there a path of connections from `from` to `to`?
        pub fn reaches(&self, from: usize, to: usize) -> bool {
            let mut seen = vec![false; self.nodes.len()];
            let mut stack = vec![from];
            while let Some(n) = stack.pop() {
                if n == to {
                    return true;
                }
                if std::mem::replace(&mut seen[n], true) {
                    continue;
                }
                stack.extend(self.edges.iter().filter(|e| e.0 == n).map(|e| e.1));
            }
            false
        }
    }

    pub type SharedGraph = Rc<RefCell<Graph>>;

    thread_local! {
        static GRAPHS: RefCell<Vec<SharedGraph>> = const { RefCell::new(Vec::new()) };
    }

    /// Every context created on this thread since [`reset_graphs`], oldest
    /// first (tests run one per thread, so this is per test).
    pub fn graphs() -> Vec<SharedGraph> {
        GRAPHS.with(|g| g.borrow().clone())
    }
    pub fn reset_graphs() {
        GRAPHS.with(|g| g.borrow_mut().clear());
    }

    fn new_graph(sample_rate: f32, offline_frames: Option<u32>) -> SharedGraph {
        let g = Rc::new(RefCell::new(Graph {
            sample_rate,
            offline_frames,
            ..Graph::default()
        }));
        // Node 0 is always the destination.
        push_node(&g, NodeKind::Destination);
        GRAPHS.with(|all| all.borrow_mut().push(Rc::clone(&g)));
        g
    }

    fn push_node(g: &SharedGraph, kind: NodeKind) -> usize {
        let mut gr = g.borrow_mut();
        gr.nodes.push(NodeRec {
            kind,
            type_name: None,
            buffer_frames: None,
            looping: false,
            start: None,
            stop: None,
        });
        gr.nodes.len() - 1
    }

    // ---- stand-ins for the wasm-bindgen glue the bake's promise code names ----

    /// `wasm_bindgen::JsValue`: never holds anything natively.
    #[derive(Debug, Clone)]
    pub struct JsValue;
    impl JsValue {
        pub fn dyn_into<T>(self) -> Result<T, JsValue> {
            Err(self)
        }
    }
    /// `wasm_bindgen::closure::Closure`: the callback is dropped, never run.
    pub struct Closure<T: ?Sized>(PhantomData<Box<T>>);
    impl<T: ?Sized> Closure<T> {
        pub fn once<F>(_f: F) -> Self {
            Closure(PhantomData)
        }
    }
    /// `js_sys::Promise`.
    pub struct Promise;
    impl Promise {
        pub fn then2<A: ?Sized, B: ?Sized>(&self, _a: &Closure<A>, _b: &Closure<B>) -> Promise {
            Promise
        }
    }

    // ---- enums ----

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OscillatorType {
        Sine,
        Square,
        Sawtooth,
        Triangle,
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BiquadFilterType {
        Lowpass,
        Highpass,
        Bandpass,
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OverSampleType {
        N2x,
    }

    // ---- nodes ----

    /// Any node: a handle into its context's [`Graph`].
    #[derive(Debug, Clone)]
    pub struct AudioNode {
        g: SharedGraph,
        pub id: usize,
    }
    impl AudioNode {
        pub fn connect_with_audio_node(&self, to: &AudioNode) -> Result<AudioNode, JsValue> {
            if !Rc::ptr_eq(&self.g, &to.g) {
                return Err(JsValue); // Web Audio throws across contexts
            }
            self.g.borrow_mut().edges.push((self.id, to.id));
            Ok(to.clone())
        }
        /// A modulation edge (an LFO into a frequency, …): recorded as an
        /// event-less connection `from -> the param's node`.
        pub fn connect_with_audio_param(&self, to: &AudioParam) -> Result<(), JsValue> {
            if !Rc::ptr_eq(&self.g, &to.g) {
                return Err(JsValue);
            }
            self.g
                .borrow_mut()
                .mod_edges
                .push((self.id, to.node, to.param));
            Ok(())
        }
        fn param(&self, param: &'static str) -> AudioParam {
            AudioParam {
                g: Rc::clone(&self.g),
                node: self.id,
                param,
            }
        }
        fn with<R>(&self, f: impl FnOnce(&mut NodeRec) -> R) -> R {
            f(&mut self.g.borrow_mut().nodes[self.id])
        }
    }
    impl AsRef<AudioNode> for AudioNode {
        fn as_ref(&self) -> &AudioNode {
            self
        }
    }

    #[derive(Debug, Clone)]
    pub struct AudioParam {
        g: SharedGraph,
        node: usize,
        param: &'static str,
    }
    impl AudioParam {
        fn push(
            &self,
            kind: EventKind,
            value: f32,
            time: f64,
            end: f64,
        ) -> Result<AudioParam, JsValue> {
            self.g.borrow_mut().events.push(ParamEvent {
                node: self.node,
                param: self.param,
                kind,
                value,
                time,
                end,
            });
            Ok(self.clone())
        }
        pub fn set_value_at_time(&self, v: f32, t: f64) -> Result<AudioParam, JsValue> {
            self.push(EventKind::Set, v, t, t)
        }
        pub fn linear_ramp_to_value_at_time(&self, v: f32, t: f64) -> Result<AudioParam, JsValue> {
            self.push(EventKind::LinearRamp, v, t, t)
        }
        pub fn exponential_ramp_to_value_at_time(
            &self,
            v: f32,
            t: f64,
        ) -> Result<AudioParam, JsValue> {
            self.push(EventKind::ExpRamp, v, t, t)
        }
        pub fn set_value_curve_at_time(
            &self,
            curve: &mut [f32],
            t: f64,
            dur: f64,
        ) -> Result<AudioParam, JsValue> {
            let peak = curve.iter().fold(0.0f32, |m, v| {
                if v.is_finite() {
                    m.max(v.abs())
                } else {
                    f32::NAN
                }
            });
            self.push(EventKind::Curve, peak, t, t + dur)
        }
        pub fn cancel_scheduled_values(&self, t: f64) -> Result<AudioParam, JsValue> {
            self.push(EventKind::Cancel, 0.0, t, t)
        }
        /// The last value this param was set / ramped to (0 before any).
        pub fn value(&self) -> f32 {
            let g = self.g.borrow();
            g.events
                .iter()
                .rev()
                .find(|e| {
                    e.node == self.node && e.param == self.param && e.kind != EventKind::Cancel
                })
                .map_or(0.0, |e| e.value)
        }
    }

    /// A typed node = a newtype over [`AudioNode`] that derefs to it (the way
    /// every `web_sys` node derefs to its parent interface).
    macro_rules! node {
        ($name:ident => $parent:ident) => {
            #[derive(Debug, Clone)]
            pub struct $name($parent);
            impl Deref for $name {
                type Target = $parent;
                fn deref(&self) -> &$parent {
                    &self.0
                }
            }
            impl AsRef<AudioNode> for $name {
                fn as_ref(&self) -> &AudioNode {
                    &self.0
                }
            }
        };
    }
    node!(AudioDestinationNode => AudioNode);
    node!(GainNode => AudioNode);
    node!(BiquadFilterNode => AudioNode);
    node!(ConvolverNode => AudioNode);
    node!(DynamicsCompressorNode => AudioNode);
    node!(WaveShaperNode => AudioNode);
    node!(DelayNode => AudioNode);
    node!(AudioScheduledSourceNode => AudioNode);
    node!(OscillatorNode => AudioScheduledSourceNode);
    node!(AudioBufferSourceNode => AudioScheduledSourceNode);

    impl AsRef<AudioScheduledSourceNode> for OscillatorNode {
        fn as_ref(&self) -> &AudioScheduledSourceNode {
            &self.0
        }
    }
    impl AsRef<AudioScheduledSourceNode> for AudioBufferSourceNode {
        fn as_ref(&self) -> &AudioScheduledSourceNode {
            &self.0
        }
    }
    impl AudioScheduledSourceNode {
        pub fn start(&self) -> Result<(), JsValue> {
            self.start_with_when(0.0)
        }
        pub fn start_with_when(&self, when: f64) -> Result<(), JsValue> {
            self.0.with(|n| n.start = Some((when, 0.0)));
            Ok(())
        }
        pub fn stop_with_when(&self, when: f64) -> Result<(), JsValue> {
            self.0.with(|n| n.stop = Some(when));
            Ok(())
        }
    }
    impl GainNode {
        pub fn gain(&self) -> AudioParam {
            self.0.param("gain")
        }
    }
    impl OscillatorNode {
        pub fn set_type(&self, t: OscillatorType) {
            self.with(|n| n.type_name = Some(format!("{t:?}")));
        }
        pub fn frequency(&self) -> AudioParam {
            self.param("frequency")
        }
    }
    impl BiquadFilterNode {
        pub fn set_type(&self, t: BiquadFilterType) {
            self.0.with(|n| n.type_name = Some(format!("{t:?}")));
        }
        pub fn frequency(&self) -> AudioParam {
            self.0.param("frequency")
        }
        pub fn q(&self) -> AudioParam {
            self.0.param("Q")
        }
    }
    impl DelayNode {
        pub fn delay_time(&self) -> AudioParam {
            self.0.param("delayTime")
        }
    }
    impl DynamicsCompressorNode {
        pub fn threshold(&self) -> AudioParam {
            self.0.param("threshold")
        }
        pub fn knee(&self) -> AudioParam {
            self.0.param("knee")
        }
        pub fn ratio(&self) -> AudioParam {
            self.0.param("ratio")
        }
        pub fn attack(&self) -> AudioParam {
            self.0.param("attack")
        }
        pub fn release(&self) -> AudioParam {
            self.0.param("release")
        }
    }
    impl WaveShaperNode {
        pub fn set_curve_opt_f32_slice(&self, _curve: Option<&mut [f32]>) {}
        pub fn set_oversample(&self, _o: OverSampleType) {}
    }
    impl ConvolverNode {
        pub fn set_buffer(&self, b: Option<&AudioBuffer>) {
            self.0.with(|n| n.buffer_frames = b.map(|b| b.frames));
        }
        pub fn set_normalize(&self, _n: bool) {}
    }
    impl AudioBufferSourceNode {
        pub fn set_buffer(&self, b: Option<&AudioBuffer>) {
            self.with(|n| n.buffer_frames = b.map(|b| b.frames));
        }
        pub fn set_loop(&self, l: bool) {
            self.with(|n| n.looping = l);
        }
        pub fn set_loop_start(&self, _t: f64) {}
        pub fn set_loop_end(&self, _t: f64) {}
        pub fn playback_rate(&self) -> AudioParam {
            self.param("playbackRate")
        }
        pub fn start_with_when_and_grain_offset(
            &self,
            when: f64,
            offset: f64,
        ) -> Result<(), JsValue> {
            self.with(|n| n.start = Some((when, offset)));
            Ok(())
        }
    }

    /// An `AudioBuffer`: only its shape is kept.
    #[derive(Debug, Clone)]
    pub struct AudioBuffer {
        pub channels: u32,
        pub frames: u32,
    }
    impl AudioBuffer {
        pub fn copy_to_channel(&self, source: &[f32], channel: i32) -> Result<(), JsValue> {
            let ok = (channel as u32) < self.channels
                && source.len() as u32 <= self.frames
                && source.iter().all(|v| v.is_finite());
            if ok {
                Ok(())
            } else {
                Err(JsValue)
            }
        }
    }

    // ---- contexts ----

    #[derive(Debug, Clone)]
    pub struct BaseAudioContext {
        g: SharedGraph,
    }
    impl BaseAudioContext {
        fn node(&self, kind: NodeKind) -> AudioNode {
            AudioNode {
                id: push_node(&self.g, kind),
                g: Rc::clone(&self.g),
            }
        }
        pub fn sample_rate(&self) -> f32 {
            self.g.borrow().sample_rate
        }
        pub fn current_time(&self) -> f64 {
            self.g.borrow().now
        }
        pub fn destination(&self) -> AudioDestinationNode {
            AudioDestinationNode(AudioNode {
                g: Rc::clone(&self.g),
                id: 0,
            })
        }
        pub fn create_gain(&self) -> Result<GainNode, JsValue> {
            Ok(GainNode(self.node(NodeKind::Gain)))
        }
        pub fn create_oscillator(&self) -> Result<OscillatorNode, JsValue> {
            Ok(OscillatorNode(AudioScheduledSourceNode(
                self.node(NodeKind::Oscillator),
            )))
        }
        pub fn create_biquad_filter(&self) -> Result<BiquadFilterNode, JsValue> {
            Ok(BiquadFilterNode(self.node(NodeKind::Biquad)))
        }
        pub fn create_buffer_source(&self) -> Result<AudioBufferSourceNode, JsValue> {
            Ok(AudioBufferSourceNode(AudioScheduledSourceNode(
                self.node(NodeKind::BufferSource),
            )))
        }
        pub fn create_dynamics_compressor(&self) -> Result<DynamicsCompressorNode, JsValue> {
            Ok(DynamicsCompressorNode(self.node(NodeKind::Compressor)))
        }
        pub fn create_wave_shaper(&self) -> Result<WaveShaperNode, JsValue> {
            Ok(WaveShaperNode(self.node(NodeKind::WaveShaper)))
        }
        pub fn create_convolver(&self) -> Result<ConvolverNode, JsValue> {
            Ok(ConvolverNode(self.node(NodeKind::Convolver)))
        }
        pub fn create_delay_with_max_delay_time(&self, _max: f64) -> Result<DelayNode, JsValue> {
            Ok(DelayNode(self.node(NodeKind::Delay)))
        }
        pub fn create_buffer(
            &self,
            channels: u32,
            frames: u32,
            sample_rate: f32,
        ) -> Result<AudioBuffer, JsValue> {
            if channels == 0 || frames == 0 || sample_rate <= 0.0 || sample_rate.is_nan() {
                return Err(JsValue); // Web Audio throws NotSupportedError
            }
            Ok(AudioBuffer { channels, frames })
        }
    }
    impl AsRef<BaseAudioContext> for BaseAudioContext {
        fn as_ref(&self) -> &BaseAudioContext {
            self
        }
    }

    macro_rules! context {
        ($name:ident) => {
            #[derive(Debug, Clone)]
            pub struct $name(BaseAudioContext);
            impl Deref for $name {
                type Target = BaseAudioContext;
                fn deref(&self) -> &BaseAudioContext {
                    &self.0
                }
            }
            impl AsRef<BaseAudioContext> for $name {
                fn as_ref(&self) -> &BaseAudioContext {
                    &self.0
                }
            }
        };
    }
    context!(AudioContext);
    context!(OfflineAudioContext);

    /// The live context's sample rate under test.
    pub const MOCK_SAMPLE_RATE: f32 = 48_000.0;

    impl AudioContext {
        #[allow(clippy::new_ret_no_self)]
        pub fn new() -> Result<AudioContext, JsValue> {
            Ok(AudioContext(BaseAudioContext {
                g: new_graph(MOCK_SAMPLE_RATE, None),
            }))
        }
        pub fn resume(&self) -> Result<Promise, JsValue> {
            Ok(Promise)
        }
        pub fn suspend(&self) -> Result<Promise, JsValue> {
            Ok(Promise)
        }
    }
    impl OfflineAudioContext {
        pub fn new_with_number_of_channels_and_length_and_sample_rate(
            channels: u32,
            frames: u32,
            sample_rate: f32,
        ) -> Result<OfflineAudioContext, JsValue> {
            if channels == 0 || frames == 0 {
                return Err(JsValue);
            }
            Ok(OfflineAudioContext(BaseAudioContext {
                g: new_graph(sample_rate, Some(frames)),
            }))
        }
        /// The graph is built and recorded; there is nothing to render.
        pub fn start_rendering(&self) -> Result<Promise, JsValue> {
            Err(JsValue)
        }
    }
}
