# rusty-chat

A basic WebSocket library, tested with client-server frame passing.

Based on the tutorial found here: http://nbaksalyar.github.io/2015/07/10/writing-chat-in-rust.html
Front end heavily based on: https://github.com/martimatix/crystal-elm-chat

## Usage
(All commands from main directory)
If you modify elm/src/chat.elm, recompile with `elm make elm/src/chat.elm --output=chat.js`
Run WebSocket server with `cargo run`
Run web server with `elm reactor`
Navigate to http://localhost:8000/elm/chat.html

TODOs:
- [DONE] Refactor into smaller modules (client, server, etc.)
- Flesh out into a lil functional chat app (works as echo server currently)
- Work in Rust States instead of current state management solution
- Update to work with latest version of Mio
