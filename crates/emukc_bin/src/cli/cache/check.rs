use clap::Args;
use emukc_internal::prelude::Kache;

#[derive(Debug, Args)]
pub(super) struct CheckArgs {
	#[arg(help = "Dry run, do not modify anything")]
	#[arg(long)]
	dry: bool,
}

pub(super) async fn exec(
	args: &CheckArgs,
	kache: &Kache,
) -> Result<(), Box<dyn std::error::Error>> {
	kache.check_all(!args.dry).await?;

	Ok(())
}
