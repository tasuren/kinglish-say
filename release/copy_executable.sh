cp -Rf target/release/bundle/osx/kinglish-say.app dist/release/kinglish-say_aarch64-apple-darwin
cp -Rf target/x86_64-apple-darwin/release/bundle/osx/kinglish-say.app dist/release/kinglish-say_x86_64-apple-darwin
cp {target/x86_64-pc-windows-msvc/release/kinglish-say.exe,wsay.exe} dist/release/kinglish-say_x86_64-pc-windows-msvc