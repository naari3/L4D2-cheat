use std::io::Write;
use std::os::raw::c_void;

use windows::Win32::System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE};

use crate::LOG_FILE;

macro_rules! write_log {
    ($($arg:tt)*) => {
        (*LOG_FILE)
            .lock()
            .unwrap()
            .write_all(format!($($arg)*).as_bytes())
            .unwrap();
        (*LOG_FILE)
            .lock()
            .unwrap().flush().unwrap();
    };
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CTakeDamageInfo {
    pub damage_force: [f32; 3],
    pub damage_position: [f32; 3],
    pub reported_position: [f32; 3],
    pub unknown36: [f32; 3],
    pub inflictor: u32,
    pub attacker: u32,
    pub weapon: u32,
    pub damage: f32,
    pub max_damage: f32,
    pub base_damage: f32,
    pub damage_type: u32,
    pub damage_custom: u32,
    pub damage_stats: u32,
    pub ammo_type: u32,
    pub radius: f32,
}

// offset: 0x4
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CBaseEntity {
    // vtable: *const *const c_void,
    pub unknown: [u8; 0xE8],
    pub health: u32,    // 0xEC
    pub life_state: u8, // 0xF0
    pub unknown2: [u8; 0x260 - 0xF0 - 1],
    pub x_speed: f32, // 0x260
    pub z_speed: f32, // 0x264
    pub y_speed: f32, // 0x268
    // TODO: x_pos offset is 0x388
    pub unknown3: [u8; 0x388 - 0x268 - 4],
    pub x_pos: f32, // 0x388
    pub z_pos: f32, // 0x38C
    pub y_pos: f32, // 0x390
}

// type OnTakeDamageAliveFn = unsafe extern "C" fn(*mut CTerrorPlayer, *const CTakeDamageInfo);
type OnTakeDamageAliveFn = unsafe extern "thiscall" fn(*mut CTerrorPlayer, *const CTakeDamageInfo);

// Your Rust function to replace the original method
unsafe extern "thiscall" fn my_rust_on_take_damage_alive(
    this: *mut CTerrorPlayer,
    info: *const CTakeDamageInfo,
) {
    if let Some(original_method) = original_on_take_damage_alive() {
        let info = CTakeDamageInfo {
            damage: 0.0,
            ..(*info).clone()
        };
        original_method(this, &info as _);
    } else {
        write_log!("Failed to call original OnTakeDamage_Alive\n");
    }
}

// Store the original method function pointer
static mut ORIGINAL_ON_TAKE_DAMAGE_ALIVE: Option<OnTakeDamageAliveFn> = None;

// Get the original method function pointer
fn original_on_take_damage_alive() -> Option<OnTakeDamageAliveFn> {
    unsafe { ORIGINAL_ON_TAKE_DAMAGE_ALIVE }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CTerrorPlayer {
    // vtable offset 490 maybe OnTakeDamage
    pub vtable: *const *const c_void,
    pub c_base_entity: CBaseEntity,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SurvivorBot {
    // vtable offset 490 maybe OnTakeDamage
    pub vtable: *const *const c_void,
    pub c_base_entity: CBaseEntity,
}

impl CTerrorPlayer {
    pub unsafe fn patch_no_damage(&mut self) {
        let vtable = self.vtable;
        let index = 292isize;

        let original_method_ptr = *vtable.offset(index) as *const ();
        let original_method =
            std::mem::transmute::<*const (), OnTakeDamageAliveFn>(original_method_ptr);
        ORIGINAL_ON_TAKE_DAMAGE_ALIVE = Some(original_method);

        let my_rust_on_take_damage_alive_ptr: *const () =
            my_rust_on_take_damage_alive as OnTakeDamageAliveFn as *const ();
        let vtable_292 = vtable.offset(index) as *mut *const c_void;
        let mut old_protection = PAGE_PROTECTION_FLAGS::default();
        let result = VirtualProtect(
            vtable.offset(index) as *mut c_void,
            std::mem::size_of::<*const c_void>(),
            PAGE_READWRITE,
            &mut old_protection,
        );
        if !result.as_bool() {
            write_log!("Failed to change vtable memory protection to read-write.\n");
            panic!("Failed to change vtable memory protection to read-write.");
        }
        *vtable_292 = my_rust_on_take_damage_alive_ptr as *const c_void;
        let mut dummy = PAGE_PROTECTION_FLAGS::default();
        let result = VirtualProtect(
            vtable.offset(index) as *mut c_void,
            std::mem::size_of::<*const c_void>(),
            old_protection,
            &mut dummy,
        );
        if !result.as_bool() {
            write_log!("Failed to change vtable memory protection to read-write.\n");
            panic!("Failed to change vtable memory protection to read-write.");
        }
    }
}
