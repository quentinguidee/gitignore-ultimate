watch:
	yarn workspace @quentinguidee/gitignore-ultimate-client run watch

.PHONY: server

server:
	cd server && cargo build
	mkdir -p client/bin
	cp server/target/debug/server client/bin/server || cp server/target/debug/server.exe client/bin/server.exe
