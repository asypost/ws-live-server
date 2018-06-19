extern crate clap;
extern crate url;
extern crate ws;

use clap::{App, Arg};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use url::Url;
use ws::util::Token;
use ws::{CloseCode, Handler, Handshake, Sender};

struct TrancodeService {
    out: Sender,
    ffmpeg: Option<Child>,
}

impl TrancodeService {
    const READ: Token = Token(1);
}

impl Handler for TrancodeService {
    fn on_open(&mut self, shake: Handshake) -> ws::Result<()> {
        if let Ok(url) = Url::parse("ws://127.0.0.1") {
            if let Ok(url) = url.join(shake.request.resource()) {
                for (name, value) in url.query_pairs() {
                    if name == "url" {
                        println!("{}", value);
                        let cmd = Command::new("ffmpeg")
                            .stdout(Stdio::piped())
                            .args(&[
                                "-i",
                                &value,
                                "-c:v",
                                "libx264",
                                "-loglevel",
                                "quiet",
                                "-threads",
                                "2",
                                "-f",
                                "flv",
                                "-ar",
                                "22050",
                                "-crf",
                                "42",
                                "-r",
                                "15",
                                "-",
                            ])
                            .spawn();
                        if let Ok(ffmpeg) = cmd {
                            self.ffmpeg = Some(ffmpeg);
                        }
                        break;
                    }
                }
            }
        }
        self.out.timeout(10, Self::READ)?;
        Ok(())
    }

    fn on_timeout(&mut self, event: Token) -> ws::Result<()> {
        if let Some(ref mut ffmpeg) = self.ffmpeg {
            if event == Self::READ {
                let mut buffer = [0_u8; 100 * 1024];
                if let Some(ref mut stdout) = ffmpeg.stdout {
                    if let Ok(size) = stdout.read(&mut buffer) {
                        if size > 0 {
                            self.out.send(&buffer[..size])?;
                            self.out.timeout(10, Self::READ)?;
                        } else {
                            self.out.close(CloseCode::Empty)?;
                        }
                    }
                }
            }
        } else {
            self.out.close(CloseCode::Protocol)?;
        }
        Ok(())
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        if self.ffmpeg.is_some() {
            let mut ffmpeg = self.ffmpeg.take().unwrap();
            if let Ok(None) = ffmpeg.try_wait() {
                let _ = ffmpeg.kill();
            }
        }
    }
}

fn main() {
    let matches = App::new("ws-live-server")
        .version("0.1.0")
        .about("Transcode video to FLV WebSocket Stream")
        .author("shell <asypost@yeah.net>")
        .arg(
            Arg::with_name("host")
                .takes_value(true)
                .help("Set listen host")
                .short("h")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("port")
                .help("Set listen port")
                .takes_value(true)
                .short("p"),
        )
        .get_matches();

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();

    let mut url = host.to_string();
    url.push_str(":");
    url.push_str(port);

    ws::listen(url, move |out| TrancodeService {
        out,
        ffmpeg: Option::None,
    }).unwrap();
}
