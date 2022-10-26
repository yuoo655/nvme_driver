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
		-serial mon:stdio \
		-display none \
		-bios default \
		-kernel os.bin \
		-drive file=nvme.img,if=none,id=nvm -device nvme,serial=deadbeef,drive=nvm 

# --trace "pci_*"

# qemu-system-riscv64 -smp 1 -machine virt -bios default -m 512M -no-reboot -serial mon:stdio -serial file:/tmp/serial.out -kernel target/riscv64/release/zcore.bin -initrd zCore/riscv64.img -append "LOG=warn" -drive file=nvme.img,if=none,id=nvm -device nvme,serial=xxxxx,drive=nvm
