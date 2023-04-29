use std::os::raw::c_void;
use std::{collections::HashMap, io::Write};

use windows::Win32::System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE};

use crate::LOG_FILE;

static mut VMT_HOOK_DICT: Option<HashMap<isize, *const ()>> = None;

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

unsafe extern "thiscall" fn on_take_damage_alive_no_damage(
    this: *mut CTerrorPlayer,
    info: *const CTakeDamageInfo,
) {
    write_log!("my_rust_on_take_damage_alive called\n");
    type A = unsafe extern "thiscall" fn(*mut CTerrorPlayer, *const CTakeDamageInfo);
    if let Some(dict) = &VMT_HOOK_DICT {
        write_log!("dict: {:?}\n", dict);
        let original = dict.get(&292isize);
        write_log!("original: {:?}\n", original);
        if let Some(original_method) = original {
            write_log!("original_method: {:?}\n", original_method);
            let original = std::mem::transmute::<*const (), A>(*original_method);
            write_log!("original: {:?}\n", original);
            let info = CTakeDamageInfo {
                damage: 0.0,
                ..(*info).clone()
            };
            write_log!("info: {:?}\n", info);
            original(this, &info as _);
            write_log!("original called\n");
        } else {
            write_log!("Failed to call original OnTakeDamage_Alive\n");
        }
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

unsafe fn vmt_hook(this: *mut CTerrorPlayer, index: isize, new_func_ptr: *const ()) {
    if VMT_HOOK_DICT.is_none() {
        VMT_HOOK_DICT = Some(HashMap::new());
    }
    write_log!("vmt_hook called\n");
    write_log!("this: {:?}\n", this);
    write_log!("index: {:?}\n", index);
    write_log!("new_func_ptr: {:?}\n", new_func_ptr);
    let vtable = (*this).vtable;
    let original_method_ptr = *vtable.offset(index) as *const ();
    write_log!("original_method_ptr: {:?}\n", original_method_ptr);
    let mut vmt_hook_dict = VMT_HOOK_DICT.take().unwrap();
    write_log!("vmt_hook_dict: {:?}\n", vmt_hook_dict);
    vmt_hook_dict.insert(index, original_method_ptr);
    write_log!("inserted vmt_hook_dict: {:?}\n", vmt_hook_dict);

    let vtable_part = vtable.offset(index) as *mut *const c_void;
    write_log!("vtable_part: {:?}\n", vtable_part);
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
    write_log!("old_protection: {:?}\n", old_protection);
    *vtable_part = new_func_ptr as *const c_void;
    write_log!("vtable_part: {:?}\n", vtable_part);
    let mut dummy = PAGE_PROTECTION_FLAGS::default();
    let result = VirtualProtect(
        vtable.offset(index) as *mut c_void,
        std::mem::size_of::<*const c_void>(),
        old_protection,
        &mut dummy,
    );
    if !result.as_bool() {
        write_log!("Failed to change vtable memory protection to read\n");
    }
}

impl CTerrorPlayer {
    pub unsafe fn patch_no_damage(&mut self) {
        let yo = on_take_damage_alive_no_damage as unsafe extern "thiscall" fn(_, _) -> _
            as *const () as *const c_void;
        write_log!("yo: {:?}\n", yo);
        vmt_hook(self, 292, yo as *const ());
        // let vtable = self.vtable;
        // let index = 292isize;

        // let original_method_ptr = *vtable.offset(index) as *const ();
        // let original_method =
        //     std::mem::transmute::<*const (), OnTakeDamageAliveFn>(original_method_ptr);
        // ORIGINAL_ON_TAKE_DAMAGE_ALIVE = Some(original_method);

        // let my_rust_on_take_damage_alive_ptr: *const () =
        //     my_rust_on_take_damage_alive as OnTakeDamageAliveFn as *const ();
        // let vtable_292 = vtable.offset(index) as *mut *const c_void;
        // let mut old_protection = PAGE_PROTECTION_FLAGS::default();
        // let result = VirtualProtect(
        //     vtable.offset(index) as *mut c_void,
        //     std::mem::size_of::<*const c_void>(),
        //     PAGE_READWRITE,
        //     &mut old_protection,
        // );
        // if !result.as_bool() {
        //     write_log!("Failed to change vtable memory protection to read-write.\n");
        //     panic!("Failed to change vtable memory protection to read-write.");
        // }
        // *vtable_292 = my_rust_on_take_damage_alive_ptr as *const c_void;
        // let mut dummy = PAGE_PROTECTION_FLAGS::default();
        // let result = VirtualProtect(
        //     vtable.offset(index) as *mut c_void,
        //     std::mem::size_of::<*const c_void>(),
        //     old_protection,
        //     &mut dummy,
        // );
        // if !result.as_bool() {
        //     write_log!("Failed to change vtable memory protection to read-write.\n");
        //     panic!("Failed to change vtable memory protection to read-write.");
        // }
    }

    pub unsafe fn jump(&mut self) {
        let jump = *(self.vtable).offset(353);
        let jump = std::mem::transmute::<_, unsafe extern "thiscall" fn()>(jump as *const ());
        jump();
    }
}
