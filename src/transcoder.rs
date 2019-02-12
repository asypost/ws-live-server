use std::io::{Error, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub enum TransCoderResponse {
    EOS,
    Error(Error),
    Data(Vec<u8>),
}

pub struct TransCoder {
    running: Arc<Mutex<bool>>,
    source: String,
    thread: Option<JoinHandle<()>>,
    sender: Sender<TransCoderResponse>,
    receiver: Receiver<TransCoderResponse>,
}

impl TransCoder {
    pub const BUFFER_SIZE: usize = 200 * 1024;

    pub fn new(src: &str) -> Self {
        let (sender, receiver) = channel();
        Self {
            running: Arc::new(Mutex::new(false)),
            source: src.to_string(),
            thread: Option::None,
            sender: sender,
            receiver: receiver,
        }
    }

    pub fn start(&mut self) {
        let mut running = self.running.lock().unwrap();
        *running = true;
        let sender = self.sender.clone();
        let src = self.source.clone();
        let running_ref = self.running.clone();
        let thread = thread::spawn(move || {
            let cmd = Command::new("ffmpeg")
                .stdout(Stdio::piped())
                .args(&[
                    "-thread_queue_size",
                    "1024",
                    "-rtsp_flags",
                    "prefer_tcp",
                    "-max_delay",
                    "500000",
                    "-stimeout",
                    "5000000",
                    "-i",
                    &src,
                    "-c:v",
                    "copy",
                    "-c:a",
                    "libmp3lame",
                    "-q:a",
                    "2",
                    "-loglevel",
                    "quiet",
                    // "-threads",
                    // "4",
                    "-f",
                    "flv",
                    "-ar",
                    "22050",
                    "-crf",
                    "50",
                    "-r",
                    "15",
                    "-",
                ])
                .spawn();
            match cmd {
                Ok(mut ffmpeg) => {
                    let mut buffer = [0_u8; Self::BUFFER_SIZE];
                    loop {
                        let running = running_ref.lock().unwrap();
                        if *running == false {
                            break;
                        }
                        drop(running);
                        if let Some(ref mut stdout) = ffmpeg.stdout {
                            if let Ok(size) = stdout.read(&mut buffer) {
                                if size > 0 {
                                    if let Err(err) = sender
                                        .send(TransCoderResponse::Data(buffer[..size].to_vec()))
                                    {
                                        eprintln!("{:?}", err);
                                        break;
                                    }
                                } else {
                                    if let Err(err) = sender.send(TransCoderResponse::EOS) {
                                        eprintln!("{:?}", err);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    if let Ok(None) = ffmpeg.try_wait() {
                        let _ = ffmpeg.kill();
                    }
                }
                Err(e) => {
                    if let Err(err) = sender.send(TransCoderResponse::Error(e)) {
                        eprintln!("{:?}", err);
                    }
                }
            }
        });
        self.thread = Some(thread);
    }

    pub fn stop(&mut self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
        drop(running);
        if self.thread.is_some() {
            let thread = self.thread.take();
            let _ = thread.unwrap().join();
        }
    }

    pub fn try_recv(&self) -> std::result::Result<TransCoderResponse, TryRecvError> {
        return self.receiver.try_recv();
    }
}
