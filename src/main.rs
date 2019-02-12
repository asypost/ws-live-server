mod transcoder;

use clap::{App, Arg};
use std::sync::mpsc::TryRecvError;
use transcoder::{TransCoder, TransCoderResponse};
use url::Url;
use ws::util::Token;
use ws::{Builder, CloseCode, Handler, Handshake, Sender, Settings};

struct TrancodeService {
    out: Sender,
    transcoder: Option<TransCoder>,
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
                        self.transcoder = Some(TransCoder::new(&value));
                        break;
                    }
                }
            }
        }
        if let Some(ref mut transcoder) = self.transcoder {
            transcoder.start();
        }
        self.out.timeout(1, Self::READ)?;
        Ok(())
    }

    fn on_timeout(&mut self, event: Token) -> ws::Result<()> {
        if event == Self::READ {
            if let Some(ref transcoder) = self.transcoder {
                let mut read_more = false;
                let mut timeout = 60;
                for _ in 0..200 {
                    let recv_result = transcoder.try_recv();
                    match recv_result {
                        Ok(response) => match response {
                            TransCoderResponse::EOS => {
                                self.out.close(CloseCode::Normal)?;
                            }
                            TransCoderResponse::Error(e) => {
                                eprintln!("{:?}", e);
                                self.out.close(CloseCode::Error)?;
                                break;
                            }
                            TransCoderResponse::Data(data) => {
                                self.out.send(data)?;
                                read_more = true;
                            }
                        },
                        Err(e) => match e {
                            TryRecvError::Empty => {
                                read_more = true;
                                timeout = 200;
                                break;
                            }
                            TryRecvError::Disconnected => {
                                self.out.close(CloseCode::Normal)?;
                                break;
                            }
                        },
                    }
                }
                if read_more {
                    self.out.timeout(timeout, Self::READ)?;
                }
            } else {
                self.out.close(CloseCode::Invalid)?;
            }
        }
        Ok(())
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        if self.transcoder.is_some() {
            let mut transcoder = self.transcoder.take().unwrap();
            transcoder.stop();
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

    Builder::new()
        .with_settings(Settings {
            out_buffer_capacity: TransCoder::BUFFER_SIZE,
            ..Settings::default()
        })
        .build(move |out| TrancodeService {
            out,
            transcoder: Option::None,
        })
        .unwrap()
        .listen(url)
        .unwrap();
}
