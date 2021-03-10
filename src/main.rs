use termion::raw::IntoRawMode;
use termion::async_stdin;
use termion::terminal_size;
use termion::color;
use std::io::{Read, Write, stdout};
use std::thread;
use std::time::Duration;
use std::fs::File;
use std::io::{BufRead, BufReader};
use clap::{Arg, App};
use unicode_segmentation::UnicodeSegmentation;

pub struct WordSplitter {
    words: Vec<String>,
    pointer: usize,
    file_obj: BufReader<File>,
}

impl<'a> WordSplitter {
    pub fn new(file_handle: &str) -> Option<Self> {
        let file_obj = match File::open(file_handle) {
            Ok(x) => x,
            Err(_) => return None,
        };
        let file_obj = BufReader::new(file_obj);

        let mut res = Self {
            words: Vec::new(),
            pointer: 0,
            file_obj
        };
        res.read_new_lines();
        Some(res)
    }

    pub fn get_next_word(&mut self) -> Option<String> {
        let res = match self.words.get(self.pointer) {
            Some(x) => x,
            None => return None,
        };
        let res = res.clone();
        self.pointer += 1;
        if self.words.len() == 100 && self.pointer == self.words.len() {
            self.read_new_lines();
        }
        Some(res)
    }

    fn read_new_lines(&mut self) {
        let mut text = String::new();
        for _ in 0..100 {
            let mut line = String::new();
            let read_res = self.file_obj.read_line(&mut line);
            if read_res.is_err() {
                println!("Error while reading file!");
                break;
            }
            text += &line;
        }
        self.words = text.split(char::is_whitespace).map(|x| x.to_string()).collect();
        self.pointer = 0;
    }
}

fn get_highlight_letter(word: &str) -> usize {
    let graphemes: Vec<&str> = word.graphemes(true).collect();
    for i in (word.len() / 5)..(word.len() / 2) {
        match graphemes[i] {
            "a" | "e" | "i" | "o" | "u" | "ä" | "ö" | "ü" => return i,
            _ => (),
        }
    }
    word.len() / 2
}

fn run() -> Result<(), ()> {
    let matches = App::new("Rusty Reader")
    .version("0.1")
    .author("Heringer <heringer@protonmail.com>")
    .about("Tool for speed reading text files")
    .arg(Arg::with_name("INPUT")
         .help("Sets the input file to use")
         .required(true)
         .index(1))
    .get_matches();

    let filename = matches.value_of("INPUT").unwrap();

    let mut reader = match WordSplitter::new(filename) {
        Some(x) => x,
        None => { eprintln!("Could not read file. Please input a valid filename."); return Err(()); }
    };

    let stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let mut stdin = async_stdin().bytes();

    let mut active = true;
    let mut word = String::new();
    let mut wpm = 200.0f64;
    let mut sentence_count = 0;

    write!(stdout,
           "{}{}{}",
           termion::cursor::Hide,
           termion::clear::All,
           termion::cursor::Goto(1, 1))
            .unwrap();

    loop {
        let b = stdin.next();
        // Clear remaining input buffer
        while stdin.next().is_some() {}

        write!(stdout, "{}", termion::clear::All).unwrap();

        if let Some(Ok(b'q')) = b {
            // Show terminal cursor again
            write!(stdout, "{}", termion::cursor::Show).unwrap();
            break;
        } else if let Some(Ok(b' ')) = b {
            active = !active;
        } else if let Some(Ok(b'+')) = b {
            if wpm < 1000.0 {
                wpm += 10.0;
            }
        } else if let Some(Ok(b'-')) = b {
            if wpm > 40.0 {
                wpm -= 10.0;
            }
        }

        if word.len() > 0 && word.chars().last().unwrap() == '.' {
            sentence_count += 1;
        }

        stdout.flush().unwrap();
        if active {
            word = match reader.get_next_word() {
                Some(x) => x,
                None => { println!("File end"); break },
            };
        }

        let mut timeout = Duration::from_secs_f64(60.0 / wpm);
        if word.len() > 6 {
            timeout += (timeout / 5) * (word.len() as u32 - 6);
        }
        if word.len() > 0 {
            timeout += match word.graphemes(true).last().unwrap() {
                "," => timeout / 2,
                "." | "?" | "!" => timeout.mul_f64(1.5),
                _ => Duration::new(0, 0),
            }
        }

        let status = match active {
            true => "> ",
            false => "||",
        };

        let (width, height) = match terminal_size() {
            Err(_) => (1, 2),
            Ok(x) => x,
        };

        let hlindex = get_highlight_letter(&word);

        // Write word
        write!(stdout, "{}{}", termion::cursor::Goto(1, height / 2), termion::clear::CurrentLine).unwrap();
        if word.graphemes(true).collect::<Vec<&str>>().len() == 1 {
            println!("{:>half$}{}{}{}", "", color::Fg(color::Red), word, color::Fg(color::Reset), half = (width / 2) as usize);
        } else if word.len() > 0 {
            let grapheme_word: Vec<&str> = word.graphemes(true).collect();
            let prefix = &grapheme_word[..hlindex].join("");
            let highlight_letter = grapheme_word[hlindex];
            let suffix = &grapheme_word[hlindex+1..].join("");
            println!("{:>half$}{}{}{}{}", prefix, color::Fg(color::Red), highlight_letter, color::Fg(color::Reset), suffix, half = (width / 2) as usize);
        }
        write!(stdout, "{}", termion::cursor::Goto(1, height / 2 - 1)).unwrap();
        println!("{:>half$}{}|{}", " ", color::Fg(color::Red), color::Fg(color::Reset), half = (width / 2) as usize);
        write!(stdout, "{}", termion::cursor::Goto(1, height / 2 + 1)).unwrap();
        println!("{:>half$}{}|{}", " ", color::Fg(color::Red), color::Fg(color::Reset), half = (width / 2) as usize);

        // Write status line
        write!(stdout, "{}{}", termion::cursor::Goto(1, 1), termion::clear::CurrentLine).unwrap();
        println!("{}\tSentence: {}\tWords per minute: {}", status, sentence_count, wpm as u32);
        thread::sleep(timeout);
    }
    // Show the cursor again before we exit.
    write!(stdout, "{}", termion::cursor::Show).unwrap();
    Ok(())
}

fn main() {
    std::process::exit(match run() {
        Ok(_) => 0,
        Err(_) => 1
    });
}
