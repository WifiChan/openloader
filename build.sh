cargo +nightly b --release -Z build-std=core,alloc
arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/openloader target/thumbv6m-none-eabi/release/openloader.bin
