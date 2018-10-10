use clap;

/// Create the CLI argument for passing the proof identifier.
pub fn id_arg<'a>() -> clap::Arg<'a, 'a> {
	clap::Arg::with_name("id")
		.long("id")
		.short("i")
		.help("the identifier of the proof to use or the new proof to create")
		.default_value("(default)")
		.takes_value(true)
}
