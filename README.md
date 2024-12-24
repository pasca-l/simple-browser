# Simple Browser
Simple browser with HTML and CSS parser, and JavaScript engine implemented from scratch.

## Requirements
- Docker 27.3.1
- Docker Compose v2.29.7

## Usage
1. Start application.
```shell
$ docker compose up
```

2. Start browser.
```shell
$ docker compose exec browser bash
(container) $ cargo run
```

3. Get HTML file from server, by accessing `http://server:8080/`.
