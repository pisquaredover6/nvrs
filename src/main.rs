use clap::Parser;
use colored::Colorize;
use std::time::{SystemTime, UNIX_EPOCH};

mod api;
pub mod config;
mod verfiles;

#[derive(Parser)]
struct Cli {
    #[arg(
        short = 'c',
        long,
        help = "Compare newver with oldver and display differences as updates"
    )]
    cmp: bool,

    #[arg(
        short = 't',
        long,
        value_name = "packages",
        help = "List of packages to update automatically, separated by a comma",
        value_delimiter = ','
    )]
    take: Option<Vec<String>>,

    #[arg(
        short = 'n',
        long,
        value_name = "packages",
        help = "List of packages to delete from the config"
    )]
    nuke: Option<Vec<String>>,

    #[arg(
        long = "config",
        value_name = "path",
        help = "Override path to the config file"
    )]
    custom_config: Option<String>,

    #[arg(long, help = "Display copyright information")]
    copyright: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.copyright {
        let current_year = 1970
            + (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs()
                / (365 * 24 * 60 * 60));

        println!(
            "Copyright (c) {} Adam Perkowski\n
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:\n
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.",
            current_year
        );
    } else if cli.cmp {
    } else if cli.take.is_some() {
    } else if cli.nuke.is_some() {
    } else {
        let config_content = config::load(cli.custom_config);
        let (_, mut newver) = verfiles::load(config_content.__config__.clone()).unwrap();

        for package in config_content.packages {
            if let Some(pkg) = newver.data.data.iter_mut().find(|p| p.0 == &package.0) {
                let latest = api::github::get_latest(package.1.github)
                    .await
                    .tag_name
                    .replacen(&package.1.prefix, "", 1);

                if pkg.1.version != latest {
                    println!(
                        "* {} {} -> {}",
                        package.0.blue(),
                        pkg.1.version.red(),
                        latest.green()
                    );
                    pkg.1.version = latest;
                    verfiles::save(newver.clone(), false, config_content.__config__.clone()).unwrap();
                } else {
                    println!("DEBUG: up to date");
                }
            } else {
                println!("DEBUG: not found");
            }
        }
    }
}

pub fn custom_error(message: &'static str, message_ext: String) {
    let mut output = format!("! {}", message.red());
    if !message_ext.is_empty() {
        output.push('\n');
        output.push_str(&message_ext);
    }
    println!("{}", output);
    std::process::exit(1);
}
