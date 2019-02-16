extern crate clap;
extern crate crypto;
extern crate ctrlc;
extern crate rexpect;

use clap::{App, Arg, SubCommand};

use rexpect::errors::*;
use rexpect::spawn;

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crypto::digest::Digest;
use crypto::sha1::Sha1;

fn sha1(s: &str) -> String {
    let mut hasher = Sha1::new();

    hasher.input_str(s);

    hasher.result_str()
}

struct AceStreamEngine {
    public_key: str,
}

fn run_cmd(cmd: &str, args: Vec<String>) -> std::process::Child {
    Command::new(cmd)
        .args(&args)
        .spawn()
        .expect("Failed to execute process")
}

impl AceStreamEngine {
    fn connect(key: &str, ace_url: &str, player: &str) {
        AceStreamEngine::start_acestream_engine();

        // Wait for engine to start up
        thread::sleep(Duration::from_millis(10000));

        AceStreamEngine::start_session(key, ace_url, player);
    }

    fn start_acestream_engine() {
        let args = vec![String::from("--client-console")];

        run_cmd("acestreamengine", args).stdout;
    }

    fn stop_acestream_engine() {
        let args = vec![String::from("acestreamengine")];

        run_cmd("pkill", args).stdout;
    }

    fn start_session(prod_key: &str, ace_url: &str, player: &str) {
        let mut p = spawn("telnet localhost 62062", None).unwrap();

        p.send_line("HELLOBG version=3");

        let line = p.exp_regex("key=.*").unwrap();
        let req_key = line.1.split(' ').collect::<Vec<_>>()[0]
            .split('=')
            .collect::<Vec<_>>()[1];

        let mut signature = req_key.to_owned() + prod_key;
        signature = sha1(&signature);

        println!("signature {}", signature);

        let resp_key = prod_key.split('-').collect::<Vec<_>>()[0].to_owned() + "-" + &signature;
        let stream_id = ace_url.split("://").collect::<Vec<_>>()[1];

        p.send_line(&format!("READY key={}", resp_key));
        p.exp_regex("AUTH.*");
        p.send_line("USERDATA [{\"gender\": \"1\"}, {\"age\": \"3\"}]");

        p.send_line(&format!("START PID {} 0", stream_id));
        let url = String::from(
            p.exp_regex("http://.*")
                .unwrap()
                .1
                .split(' ')
                .collect::<Vec<_>>()[0]
                .to_owned(),
        );

        let args = vec![String::from(url)];
        let player_process = run_cmd(&format!("{}", player), args)
            .wait()
            .expect("Failed to start player");
    }
}

fn set_term_handler(running: &Arc<AtomicBool>) {
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

fn main() {
    let matches = App::new("Rusty AcePlayer")
        .version("1.0")
        .author("Fahad Saleh <ecsumed@yahoo.com>")
        .about("Plays an acestream on most players.")
        .arg(
            Arg::with_name("player")
                .short("p")
                .long("player")
                .help("Name of the player binary, for example: 'vlc', default 'smplayer'")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("stream")
                .short("s")
                .help("URL of the stream to play")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("key")
                .short("k")
                .help("Optional key to use.")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    // Start loop and terminated on <Ctrl-C>
    let running = Arc::new(AtomicBool::new(true));
    set_term_handler(&running);

    let player = matches.value_of("player").unwrap_or("smplayer");

    let stream = matches.value_of("stream").unwrap();

    let key = matches
        .value_of("key")
        .unwrap_or("kjYX790gTytRaXV04IvC-xZH3A18sj5b1Tf3I-J5XVS1xsj-j0797KwxxLpBl26HPvWMm");

    println!("Player: {}", player);
    println!("Stream: {}", stream);
    println!("Key: {}", key);

    AceStreamEngine::connect(key, stream, player);

    println!("Waiting for Ctrl-C...");
    while running.load(Ordering::SeqCst) {}

    AceStreamEngine::stop_acestream_engine();
    println!("Got it! Exiting...");
}
