use clap::Parser;
use log::error;


fn main() {
    let args = lilscript::ArgumentParser::parse();
    args.set_log_level();

    if let Err(e) = lilscript::run(args) {
        error!("{}", e);
    }
}
