#!/bin/bash

readonly TARGET_USER=node
readonly TARGET_PATH=/home/${TARGET_USER}/logs
readonly DEST_BASE_PATH=./logs

copy_logs() {
    local target_host=$1
    local target_address=${TARGET_USER}@${target_host}.local
    local dest_path=${DEST_BASE_PATH}/${target_host}

    echo "------COPY-------"
    echo "Copying logs from Raspberry Pi at ${target_host} to ${dest_path}..."
    rsync -avz --progress ${target_address}:${TARGET_PATH} ${dest_path}
    echo "Copy complete."
}

delete_logs() {
    local target_host=$1
    local target_address=${TARGET_USER}@${target_host}.local

    echo "------DELETE-------"
    echo "Deleting logs directory on Raspberry Pi at ${target_host}..."
    ssh ${target_address} "rm -rf ${TARGET_PATH}"
    echo "Logs directory deleted."
}

# Loop through each passed hostname
for target_host in "$@"
do
    if [ "$target_host" == "delete" ]; then
        # Skip if the argument is 'delete'
        continue
    fi

    # Perform operations
    copy_logs $target_host
    if [[ "$@" =~ " delete" ]]; then
        delete_logs $target_host
    fi
done
