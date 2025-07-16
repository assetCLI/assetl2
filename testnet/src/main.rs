use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let (supply, sig) = testnet::run_demo()?;
    println!("Final supply: {supply}");
    println!("Posted batch with tx: {sig}");
    Ok(())
}
