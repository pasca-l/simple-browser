# Simple Browser
Simple browser with HTML and CSS parser, and JavaScript engine implemented from scratch.

## Requirements
- Docker 27.3.1
- Docker Compose v2.29.7

## Usage
1. Start application, and enter container.
```shell
$ docker compose up && docker compose exec browser bash
```

2. Start browser.
- open with TUI (Terminal User Interface)
```shell
(container) $ cargo run --bin="cui_browser" --features="cui"
```

- open with GUI (Graphical User Interface)
```shell
(container) $ cargo run --bin="gui_browser" --features="gui"
```

3. Get HTML file from server, by accessing `http://server:8080/`.
