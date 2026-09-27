fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let code = if args.is_empty() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .expect("runtime");
    match runtime.block_on(ham_radio_sniffer::serve_from_env()) {
      Ok(()) => 0,
      Err(error) => {
        eprintln!("Sniffer error: {error}");
        1
      }
    }
  } else {
    ham_radio_sniffer::run_cli(&args)
  };

  std::process::exit(code);
}
