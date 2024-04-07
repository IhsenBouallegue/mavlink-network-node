# PowerShell Version

$TARGET_USER = "node"
$TARGET_PATH = "/home/${TARGET_USER}/logs"
$DEST_BASE_PATH = "./logs"

Function Copy-Logs {
    param (
        [string]$target_host
    )

    $target_address = "${TARGET_USER}@${target_host}.local"
    $dest_path = "${DEST_BASE_PATH}/${target_host}"

    Write-Host "------COPY-------"
    Write-Host "Copying logs from Raspberry Pi at ${target_host} to ${dest_path}..."
    & scp -r "${target_address}:${TARGET_PATH}" "${dest_path}"
    Write-Host "Copy complete."
}

Function Delete-Logs {
    param (
        [string]$target_host
    )

    $target_address = "${TARGET_USER}@${target_host}.local"

    Write-Host "------DELETE-------"
    Write-Host "Deleting logs directory on Raspberry Pi at ${target_host}..."
    & ssh "${target_address}" "rm -rf ${TARGET_PATH}"
    Write-Host "Logs directory deleted."
}

# Loop through each passed hostname
$args | ForEach-Object {
    $target_host = $_

    if ($target_host -eq "delete") {
        # Skip if the argument is 'delete'
        return
    }

    # Perform operations
    Copy-Logs -target_host $target_host
    if ($args -contains "delete") {
        Delete-Logs -target_host $target_host
    }
}
