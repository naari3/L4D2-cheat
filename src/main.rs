use hudhook::inject::Process;

fn main() {
    Process::by_name("left4dead2.exe")
        .unwrap()
        .inject("target\\release\\hello_hud.dll".into())
        .unwrap();
}
