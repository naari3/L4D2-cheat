#![feature(abi_thiscall)]
#![windows_subsystem = "windows"]
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

use __core::ffi::{c_char, CStr};
use hudhook::hooks::dx9::ImguiDx9Hooks;
use hudhook::hooks::{ImguiRenderLoop, ImguiRenderLoopFlags};
use imgui::*;
use l4d2_structs::{CTerrorPlayer, SurvivorBot};

use once_cell::sync::Lazy;
use windows::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32First, Module32Next, MODULEENTRY32, TH32CS_SNAPMODULE,
    TH32CS_SNAPMODULE32,
};
use windows::Win32::System::Memory::{
    VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_J};

mod l4d2_structs;

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

pub static LOG_FILE: Lazy<Mutex<File>> = Lazy::new(|| {
    let mut log_file =
        File::create("C:\\Users\\naari\\src\\github.com\\naari3\\hello-hud\\hello_hud.txt")
            .unwrap();
    log_file
        .write_all(format!("Hello, world!\n").as_bytes())
        .unwrap();

    Mutex::new(log_file)
});

macro_rules! w {
    ($f:ident($($content:tt)*)) => {
        if $f($($content)*) == false {
            let err = GetLastError();
            let error_str = format!(
                "{} (line {}) failed with error code {:?}",
                stringify!(f), line!(), err
            );
            return Err(String::from(error_str).into())
        }
    };
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
unsafe fn get_module_base_address(pid: u32, mod_name: &str) -> Result<u32> {
    let ss = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid)?;
    if ss == INVALID_HANDLE_VALUE {
        return Err(String::from("Could not get snapshot").into());
    }
    let mut entry = MODULEENTRY32 {
        dwSize: std::mem::size_of::<MODULEENTRY32>() as u32,
        ..Default::default()
    };
    let addr;
    w!(Module32First(ss, &mut entry));
    loop {
        let mut sz_module: [c_char; 256] = std::mem::transmute_copy(&entry.szModule);
        let entry_mod_name = CStr::from_ptr(sz_module.as_mut_ptr())
            .to_string_lossy()
            .into_owned();
        if entry_mod_name == mod_name {
            addr = entry.modBaseAddr as u32;
            break;
        }
        w!(Module32Next(ss, &mut entry));
    }

    w!(CloseHandle(ss));
    if addr == 0 {
        return Err(String::from("Could not get module name").into());
    }

    Ok(addr)
}

struct L4D2Hud {
    start_time: Instant,
    ammo: i32,
    infinity_health: bool,
    server_base_address: u32,
}

impl L4D2Hud {
    fn new() -> Self {
        unsafe {
            FreeConsole();
            AttachConsole(ATTACH_PARENT_PROCESS);
        }

        let server_base_address =
            unsafe { get_module_base_address(std::process::id(), "server.dll").unwrap() };

        // write base_address as hex to file();
        write_log!("{:x}\n", server_base_address);

        Self {
            start_time: Instant::now(),
            server_base_address,
            ammo: 0,
            infinity_health: false,
        }
    }
}

unsafe fn get_address(base_address: u32, offsets: Vec<u32>) -> u32 {
    let mut address = *(base_address as *mut u32);
    for (i, &offset) in offsets.iter().enumerate() {
        if i == offsets.len() - 1 {
            address += offset;
        } else {
            address = *((address + offset) as *mut u32);
        }
    }
    address
}

unsafe fn get_address_mut<T>(base_address: u32, offsets: Vec<u32>) -> *mut T {
    get_address(base_address, offsets) as *mut T
}

unsafe fn patch(address: u32, body: Vec<u8>) {
    let mut old_protection = PAGE_PROTECTION_FLAGS::default();
    VirtualProtect(
        address as *mut _,
        body.len(),
        PAGE_EXECUTE_READWRITE,
        &mut old_protection,
    );
    for (i, p) in body.into_iter().enumerate() {
        *((address + i as u32) as *mut u8) = p;
    }
}

impl ImguiRenderLoop for L4D2Hud {
    fn render(&mut self, ui: &mut Ui, _flags: &ImguiRenderLoopFlags) {
        ui.window("##hello")
            .size([320., 250.], Condition::Always)
            .build(|| unsafe {
                let player =
                    get_address_mut::<CTerrorPlayer>(self.server_base_address + 0x7DD774, vec![]);
                let bot1 =
                    get_address_mut::<SurvivorBot>(self.server_base_address + 0x7DD784, vec![]);
                let bot2 =
                    get_address_mut::<SurvivorBot>(self.server_base_address + 0x7DD794, vec![]);
                let bot3 =
                    get_address_mut::<SurvivorBot>(self.server_base_address + 0x7DD7A4, vec![]);

                ui.text(format!("Elapsed: {:?}", self.start_time.elapsed()));
                ui.text(format!("X Speed: {}", (*player).c_base_entity.x_speed));
                ui.text(format!("Y Speed: {}", (*player).c_base_entity.y_speed));
                ui.text(format!("Z Speed: {}", (*player).c_base_entity.z_speed));

                ui.text(format!("X Pos: {}", (*player).c_base_entity.x_pos));
                ui.text(format!("Y Pos: {}", (*player).c_base_entity.y_pos));
                ui.text(format!("Z Pos: {}", (*player).c_base_entity.z_pos));
                if ui.button("Infinity Ammo") {
                    patch(
                        self.server_base_address + 0x3E6140,
                        vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
                    );
                }
                if ui.button("Infinity Chainsaw") {
                    patch(
                        self.server_base_address + 0x3C76DD,
                        vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
                    );
                }
                if ui.checkbox("Infinity HP", &mut self.infinity_health) {}
                if self.infinity_health {
                    let player = get_address_mut::<CTerrorPlayer>(
                        self.server_base_address + 0x7DD774,
                        vec![],
                    );
                    (*player).c_base_entity.health = 99999;
                }
                if ui.button("Reech to floor") {
                    (*player).c_base_entity.y_speed = 9999.0f32;
                }
                if ui.button("Hyper Jump") || GetAsyncKeyState(VK_J.0 as _) == -32768 {
                    (*player).c_base_entity.y_speed = 999.0f32;
                    (*player).c_base_entity.x_speed *= 3.0f32;
                    (*player).c_base_entity.z_speed *= 3.0f32;
                }
                if ui.button("Bot tp to you") {
                    (*bot1).c_base_entity.x_pos = (*player).c_base_entity.x_pos;
                    (*bot1).c_base_entity.y_pos = (*player).c_base_entity.y_pos;
                    (*bot1).c_base_entity.z_pos = (*player).c_base_entity.z_pos;

                    (*bot2).c_base_entity.x_pos = (*player).c_base_entity.x_pos;
                    (*bot2).c_base_entity.y_pos = (*player).c_base_entity.y_pos;
                    (*bot2).c_base_entity.z_pos = (*player).c_base_entity.z_pos;

                    (*bot3).c_base_entity.x_pos = (*player).c_base_entity.x_pos;
                    (*bot3).c_base_entity.y_pos = (*player).c_base_entity.y_pos;
                    (*bot3).c_base_entity.z_pos = (*player).c_base_entity.z_pos;
                }
                if ui.button("Kill all bots") {
                    (*bot1).c_base_entity.health = 0;
                    (*bot2).c_base_entity.health = 0;
                    (*bot3).c_base_entity.health = 0;
                    (*bot1).c_base_entity.life_state = 1;
                    (*bot2).c_base_entity.life_state = 1;
                    (*bot3).c_base_entity.life_state = 1;
                }
                if ui.button("No damage") {
                    (*player).patch_no_damage();
                }
                if ui.input_int("Ammo", &mut self.ammo).build() {
                    *get_address_mut(
                        self.server_base_address + 0x8376B8,
                        vec![0x0, 0x28, 0xC, 0x1414],
                    ) = self.ammo;
                }
            });
    }
}

hudhook::hudhook!(L4D2Hud::new().into_hook::<ImguiDx9Hooks>());
