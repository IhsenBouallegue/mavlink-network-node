#!/bin/bash

readonly TARGET_HOST=$1
readonly TARGET_USER=node
readonly TARGET_PATH=/home/${TARGET_USER}/logs/&{TARGET_HOST}
readonly TARGET_ADDRESS=${TARGET_USER}@${TARGET_HOST}.local

readonly DEST_PATH=./


copy_logs() {
    echo "Copying logs from Raspberry Pi..."
    rsync -avz --progress ${TARGET_ADDRESS}:${TARGET_PATH} ${DEST_PATH}
    echo "Copy complete."
}

delete_logs() {
    echo "Deleting logs directory on Raspberry Pi..."
    ssh ${TARGET_ADDRESS} "rm -rf ${TARGET_PATH}"
    echo "Logs directory deleted."
}

if [ "$2" == "delete" ]; then
    copy_logs
    delete_logs
else
    copy_logs
fi