# ws-live-server
Transcode video to FLV WebSocket Stream
# Purpose
The original purpose of this repo is provide another way to play rstp stream in modern web browsers instead of using an IE only ActiveX plugin.
In my case it's playing the role of the ActiveX plugin so it's running on local machine and [flv.js](https://github.com/Bilibili/flv.js) is used as the Html5 Video Player.
# Build
cargo build --release
# Run / Test
cargo run -- -h 127.0.0.1 -p 7788

