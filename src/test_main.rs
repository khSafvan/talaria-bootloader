
use uefi::prelude::*;
use uefi::boot;
#[entry]
fn main() -> Status {
    let handle = boot::image_handle();
    Status::SUCCESS
}

