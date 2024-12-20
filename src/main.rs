use colored::Colorize;
use nvrs::*;

mod cli;

#[tokio::main]
async fn main() -> error::Result<()> {
    match init().await {
        Ok(core) => {
            let res = if core.1.cmp {
                compare(core.0).await
            } else if core.1.take.is_some() {
                take(core.0, core.1.take).await
            } else if core.1.nuke.is_some() {
                nuke(core.0, core.1.nuke, core.1.no_fail).await
            } else {
                sync(core.0, core.1.no_fail).await
            };

            match res {
                Ok(_) => (),
                Err(e) => pretty_error(&e),
            }
        }
        Err(e) => pretty_error(&e),
    }

    Ok(())
}

async fn init() -> error::Result<(Core, cli::Cli)> {
    let cli = cli::get_args();
    let config = config::load(cli.clone().custom_config).await?;

    let verfiles = verfiles::load(config.0.__config__.clone()).await?;
    let keyfile = keyfile::load(config.0.__config__.clone()).await?;

    Ok((
        Core {
            config,
            verfiles,
            client: reqwest::Client::new(),
            keyfile,
        },
        cli,
    ))
}

async fn compare(core: Core) -> error::Result<()> {
    let (oldver, newver) = core.verfiles;

    for new_pkg in newver.data.data {
        if let Some(old_pkg) = oldver.data.data.iter().find(|p| p.0 == &new_pkg.0) {
            if old_pkg.1.version != new_pkg.1.version {
                println!(
                    "{} {} {} -> {}",
                    "*".white().on_black(),
                    new_pkg.0.blue(),
                    old_pkg.1.version.red(),
                    new_pkg.1.version.green()
                );
            }
        } else {
            println!(
                "{} {} {} -> {}",
                "*".white().on_black(),
                new_pkg.0.blue(),
                "NONE".red(),
                new_pkg.1.version.green()
            );
        }
    }

    Ok(())
}

async fn take(core: Core, take_names: Option<Vec<String>>) -> error::Result<()> {
    let names = take_names.unwrap();
    let config = core.config;
    let (mut oldver, newver) = core.verfiles;

    let packages_to_update = if names.contains(&"ALL".to_string()) {
        newver.data.data.keys().cloned().collect()
    } else {
        names
    };

    for package_name in packages_to_update {
        if let Some(new_pkg) = newver.data.data.get(&package_name) {
            if let Some(old_pkg) = oldver.data.data.get_mut(&package_name) {
                if old_pkg.version != new_pkg.version {
                    println!(
                        "{} {} {} -> {}",
                        "+".white().on_black(),
                        package_name.blue(),
                        old_pkg.version.red(),
                        new_pkg.version.green()
                    );
                    old_pkg.version = new_pkg.version.clone();
                    old_pkg.gitref = new_pkg.gitref.clone();
                    old_pkg.url = new_pkg.url.clone();
                }
            } else {
                println!(
                    "{} {} {} -> {}",
                    "+".white().on_black(),
                    package_name.blue(),
                    "NONE".red(),
                    new_pkg.version.green()
                );
                oldver.data.data.insert(package_name, new_pkg.clone());
            }
        } else {
            return Err(error::Error::PkgNotInNewver(package_name));
        }
    }

    verfiles::save(oldver, true, config.0.__config__).await
}

async fn nuke(core: Core, nuke_names: Option<Vec<String>>, no_fail: bool) -> error::Result<()> {
    let names = nuke_names.unwrap();
    let mut config_content = core.config.0;
    let (mut oldver, mut newver) = core.verfiles;

    for package_name in names {
        if config_content.packages.contains_key(&package_name) {
            config_content.packages.remove(&package_name);
        } else if no_fail {
            pretty_error(&error::Error::PkgNotInConfig(package_name.clone()));
        } else {
            return Err(error::Error::PkgNotInConfig(package_name));
        }
        newver.data.data.remove(&package_name);
        oldver.data.data.remove(&package_name);
    }

    verfiles::save(newver, false, config_content.__config__.clone()).await?;
    verfiles::save(oldver, true, config_content.__config__.clone()).await?;
    config::save(config_content, core.config.1).await?;

    Ok(())
}

async fn sync(core: Core, no_fail: bool) -> error::Result<()> {
    let config = core.config.0;
    let (_, mut newver) = core.verfiles;

    let tasks: Vec<_> = config
        .packages
        .clone()
        .into_iter()
        .map(|pkg| tokio::spawn(run_source(pkg, core.client.clone(), core.keyfile.clone())))
        .collect();

    let mut results = futures::future::join_all(tasks).await;

    for package in config.packages {
        match results.remove(0).unwrap() {
            Ok(release) => {
                if let Some(new_pkg) = newver.data.data.iter_mut().find(|p| p.0 == &package.0) {
                    let gitref: String;
                    let tag = if let Some(t) = release.tag.clone() {
                        gitref = format!("refs/tags/{}", t);
                        release.tag.unwrap().replacen(&package.1.prefix, "", 1)
                    } else {
                        gitref = String::new();
                        release.name
                    };

                    if new_pkg.1.version != tag {
                        println!(
                            "{} {} {} -> {}",
                            "|".white().on_black(),
                            package.0.blue(),
                            new_pkg.1.version.red(),
                            tag.green()
                        );
                        new_pkg.1.version = tag.clone();
                        new_pkg.1.gitref = gitref;
                        new_pkg.1.url = release.url;
                    }
                } else {
                    let gitref: String;
                    let tag = if let Some(t) = release.tag.clone() {
                        gitref = format!("refs/tags/{}", t);
                        release.tag.unwrap().replacen(&package.1.prefix, "", 1)
                    } else {
                        gitref = String::new();
                        release.name
                    };

                    println!(
                        "{} {} {} -> {}",
                        "|".white().on_black(),
                        package.0.blue(),
                        "NONE".red(),
                        tag.green()
                    );
                    newver.data.data.insert(
                        package.0,
                        verfiles::VerPackage {
                            version: tag.clone(),
                            gitref,
                            url: release.url,
                        },
                    );
                }
            }
            Err(e) => {
                if !no_fail {
                    return Err(e);
                } else {
                    pretty_error(&e);
                }
            }
        };
    }

    verfiles::save(newver, false, config.__config__).await
}

fn pretty_error(err: &error::Error) {
    let mut lines: Vec<String> = err
        .to_string()
        .lines()
        .map(|line| line.to_string())
        .collect();
    let first = lines.remove(0);
    let first_split = first.split_once(':').unwrap_or(("", &first));
    if first_split.0.is_empty() {
        println!("{} {}", "!".red().bold().on_black(), first_split.1.red());
    } else {
        println!(
            "{} {}:{}",
            "!".red().bold().on_black(),
            first_split.0,
            first_split.1.red()
        );
    }
    for line in lines {
        println!("{}  {}", "!".red().on_black(), line)
    }
}

#[tokio::test]
async fn core_initializing() {
    assert!(init().await.is_ok())
}
