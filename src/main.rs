use clap::Parser;
use std::time::{SystemTime, UNIX_EPOCH};

mod config;

#[derive(Parser)]
struct Cli {
    #[arg(
        short = 'c',
        long,
        help = "Compare the newver with oldver and display differences as updates"
    )]
    cmp: bool,

    #[arg(long, help = "Display copyright information")]
    copyright: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.copyright {
        let current_year = 1970
            + (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
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
    }
}
