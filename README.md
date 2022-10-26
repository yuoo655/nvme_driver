# nvme drivers
nvme drivers for riscv64 on Qemu and fu740 board

## example run

```
cd example
dd if=/dev/zero bs=1M count=16 of=nvme.img
make qemu-nvme
cat | head -c 1024 nvme.img | xxd
```
