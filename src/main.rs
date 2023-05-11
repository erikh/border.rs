use border::config::Config;

fn main() -> Result<(), anyhow::Error> {
    let mut f = std::fs::OpenOptions::new();
    f.read(true);
    let io = f.open("example.yaml")?;
    let res: Config = serde_yaml::from_reader(io)?;
    println!("{:?}", res);
    Ok(())
}
