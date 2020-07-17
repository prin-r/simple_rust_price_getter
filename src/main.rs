pub mod band;

use band::*;
use tokio::runtime::Runtime;

fn main() {
    let x = BandSource::new(1, "0000000442414e4400000000000f4240".into(), 4, 4);

    let result = Runtime::new()
        .expect("Failed to create Tokio runtime")
        .block_on(x.request_data());

    println!("{:?}", result);
}
