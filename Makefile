gitignore-watch:
	yarn workspace @quentinguidee/gitignore-ultimate-vscode run watch

.PHONY: gitignore-server

gitignore-server:
	cargo build -p gitignore-ultimate-server
	mkdir -p clients/vscode-gitignore/bin
	cp target/debug/gitignore-ultimate-server clients/vscode-gitignore/bin/server || cp target/debug/gitignore-ultimate-server.exe clients/vscode-gitignore/bin/server.exe
