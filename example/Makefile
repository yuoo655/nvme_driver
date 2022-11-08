env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview
	
build:
	cargo build --release

osbin: build
	rust-objcopy --binary-architecture=riscv64 target/riscv64gc-unknown-none-elf/release/example -O binary os.bin

clean:
	cargo clean

qemu: 
	qemu-system-riscv64 \
		-machine virt \
		-serial mon:stdio \
		-bios default \
		-kernel os.bin


qemu-nvme: osbin
	qemu-system-riscv64 \
		-machine virt \
		-serial stdio \
		-display none \
		-bios default \
		-kernel os.bin \
		-drive file=nvme.img,if=none,id=nvm \
		-device nvme,serial=deadbeef,drive=nvm

# -drive file=nvme.img,if=none,id=nvm,format=raw -device nvme,serial=deadbeef,drive=nvm



# qemu-system-riscv64 -smp 1 -machine virt -bios default -m 512M -no-reboot -serial mon:stdio -serial file:/tmp/serial.out -kernel target/riscv64/release/zcore.bin -initrd zCore/riscv64.img -append "LOG=warn" -drive file=nvme.img,if=none,id=nvm -device nvme,serial=xxxxx,drive=nvm

# 5.21.1.8 Interrupt Coalescing (Feature Identifier 08h)