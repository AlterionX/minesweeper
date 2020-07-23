use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    width: usize,
    height: usize,
    mines: u64,
    preset: String,
}
