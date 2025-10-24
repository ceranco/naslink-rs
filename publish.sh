cd "$(dirname "$0")"
cargo build --release
mkdir -p target/publish
rm -rf target/publish/*

cp target/release/naslink-rs target/publish/
cp -r wwwroot target/publish/wwwroot