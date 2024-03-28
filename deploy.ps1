$ErrorActionPreference = "Stop"
Set-PSDebug -Trace 1

$TARGET_ADDRESS = $args[0]
$NODE_TYPE = $args[1]
$EXAMPLE = $args[2]

$TARGET_USER = "node"
$TARGET_HOST = "${TARGET_USER}@${TARGET_ADDRESS}"

$TARGET_PATH = "/home/${TARGET_USER}/${EXAMPLE}"
$TARGET_ARCH = "aarch64-unknown-linux-gnu"
$SOURCE_PATH = "./target/${TARGET_ARCH}/release/examples/${EXAMPLE}"

# Dynamically determine the project directory in WSL path format
$windowsPath = Get-Location
$wslPath = "/mnt/$($windowsPath.Path.ToLower().Replace(':', '').Replace('\', '/'))"

# Building in WSL
wsl --exec bash -lc "cd $wslPath && cargo build --release --example $EXAMPLE --target=$TARGET_ARCH"

# Deploying with rsync and running with ssh from PowerShell
scp -r $SOURCE_PATH ${TARGET_HOST}:${TARGET_PATH}
ssh -t ${TARGET_HOST} "chmod +x ${TARGET_PATH} && RUST_LOG=debug ${TARGET_PATH} ${NODE_TYPE}"

