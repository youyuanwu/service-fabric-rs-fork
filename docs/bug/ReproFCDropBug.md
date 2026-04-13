# Repro steps:

Setup DevContainer using vscode choosing azl3 and then run these steps:
```sh
# Build and deploy apps:
cmake . -DCMAKE_BUILD_TYPE=Debug -B build
cmake --build build --config Debug
bash ./scripts/prepare_test_apps.sh 

# Run the test repeatedly until it crashes
retry=0
while cargo test -p samples_reflection -- test_resolve_notification --nocapture; do
	retry=$((retry + 1))
	echo "Retry #${retry}: test passed, running again..."
done
```