use std::process::ExitCode;

use marlow::Kvm;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("marlow: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> marlow::Result<()> {
    let kvm = Kvm::new()?;
    println!("opened /dev/kvm, API version OK");

    let _vm = kvm.create_vm()?;
    println!("created VM");

    Ok(())
}