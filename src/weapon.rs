pub enum WeaponType {
    Pistol,
    Shotgun,
    MachineGun,
    Melee,
}

pub struct Weapon {
    pub weapon_type: WeaponType,
    pub damage: i32,
    pub ammo: i32,
    pub max_ammo: i32,
    pub fire_rate: f32,
}

impl Weapon {
    pub fn new(weapon_type: WeaponType) -> Self {
        match weapon_type {
            WeaponType::Pistol => Self {
                weapon_type,
                damage: 50,
                ammo: 12,
                max_ammo: 12,
                fire_rate: 0.5,
            },
            WeaponType::Shotgun => Self {
                weapon_type,
                damage: 80,
                ammo: 6,
                max_ammo: 6,
                fire_rate: 1.0,
            },
            WeaponType::MachineGun => Self {
                weapon_type,
                damage: 30,
                ammo: 30,
                max_ammo: 30,
                fire_rate: 0.1,
            },
            WeaponType::Melee => Self {
                weapon_type,
                damage: 100,
                ammo: 999,
                max_ammo: 999,
                fire_rate: 0.5,
            },
        }
    }
}
