# Compat S2 Initrd Builder

This builds a minimal initrd that prints `COMPAT_S2: hello`, mounts a
read-only artifact disk, and validates read/write behavior.

## Build
```
./tools/compat/build_compat_initrd.sh
```

## Run gate
Set a Linux kernel image, the initrd output, and an artifact image:
```
S2_COMPAT_KERNEL=/path/to/bzImage \
S2_COMPAT_INITRD=out/compat_s2/initrd.cpio.gz \
S2_COMPAT_ARTIFACT=out/compat_s2/artifact.img \
./tools/ci/foundry_compat_s2.sh
```

Notes:
- Requires a static-capable C compiler (e.g., `gcc -static` or `musl-gcc`).
- The kernel image is not provided by this repo.
- Artifact image is built by `tools/compat/build_compat_artifact_img.sh`.
- Uses `debugfs` to write into the ext4 image without mounting it.
