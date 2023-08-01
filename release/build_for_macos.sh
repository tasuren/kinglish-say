cargo bundle --release
cargo bundle --release --target x86_64-apple-darwin
cargo xwin build --release --target x86_64-pc-windows-msvc

mkdir -p dist/release/{kinglish-say_x86_64-apple-darwin,kinglish-say_aarch64-apple-darwin,kinglish-say_x86_64-pc-windows-msvc}

./release/copy_executable.sh
./release/copy_misc.sh