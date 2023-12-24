# llame

A minimal desktop command-line application providing a user friendly way to interact with Ollama APIs.

## Install

For now, the only way to formally install this app is to use run `cargo install llame`.
This requires Rust to be set up on your computer. You may also run this from source.

## Setup

Create the following folder structure

```text
<root>
├── chat_about_cats
├── ...
├── homework_help
└── config.toml
```

`<root>` can be any name you want.

The `chat_about_cats` and `homework_help` folders are example names for chats. Indeed, chats are represented by folders, and the LLM's memory is stored in the folder itself under the name `context.dat`. Deleting this file before re-running this app allows you to wipe the LLM's memory while re-using the name of the chat. Replacing and swapping this file is valid, but the file will only be read when the `chat` command is first ran. Using a context from one model with another will probably not work unless both models accept the same sized context. If you get an example to work please tell me!

The folder will also contain `system.txt`, which is auto-generated if it does not exist. You may write a system message in here at any point in time, even while the app is running and it will be hot-reloaded (Do keep in mind LLMs will still follow old system messages until they forget about them).

`config.toml` is a mandatory file containing information regarding the model and the API. Here is a full example:

```toml
model = "dolphin-mixtral:latest"
host = "https://manglemix.ngrok.io"
port = 443
```

The `host` and `port` parameters are not required. They will default to `http://127.0.0.1` and `11434` respectively. As shown in this example, `https` is also allowed and is
highly recommended if possible to keep your conversations private. I use `ngrok` if my Ollama API is running remotely as it automatically provides `https`.

To start a chat, enter a chat folder and run the `llame chat` command. All folders in the same directory as `config.toml` is a valid chat folder.

To summarize a chat, enter a chat folder and run the `llame summary` command. This will send the following prompt to the LLM with the current system message and context: "Briefly summarize this conversation".
