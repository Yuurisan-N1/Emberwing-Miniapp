#!/usr/bin/env bash

BINARY="./target/release/emberwing-bot"
LOG_FILE="emberwing-bot.log"
SESSION_NAME="emberwing-bot"


build_release() {
    echo "Building release binary..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Build failed, aborting."
        exit 1
    fi
}


mode_direct() {
    build_release
    exec "$BINARY"
}

mode_nohup() {
    build_release
    nohup "$BINARY" >> "$LOG_FILE" 2>&1 &
    echo "Started in background (PID $!)"
    echo "Logs: tail -f $LOG_FILE"
}

mode_screen() {
    if ! command -v screen &>/dev/null; then
        echo "screen not found, install with: apt install screen"
        exit 1
    fi
    build_release
    screen -dmS "$SESSION_NAME" "$BINARY"
    echo "Started in screen session '$SESSION_NAME'"
    echo "Attach : screen -r $SESSION_NAME"
    echo "Detach : Ctrl+A then D"
}

mode_tmux() {
    if ! command -v tmux &>/dev/null; then
        echo "tmux not found, install with: apt install tmux"
        exit 1
    fi
    build_release
    tmux new-session -d -s "$SESSION_NAME" "$BINARY"
    echo "Started in tmux session '$SESSION_NAME'"
    echo "Attach : tmux attach -t $SESSION_NAME"
    echo "Detach : Ctrl+B then D"
}

mode_logs() {
    tail -f "$LOG_FILE"
}

mode_stop() {
    pkill -f "$BINARY" && echo "Stopped." || echo "No running process found."
}


case "${1:-direct}" in
    direct) mode_direct ;;
    nohup)  mode_nohup ;;
    screen) mode_screen ;;
    tmux)   mode_tmux ;;
    logs)   mode_logs ;;
    stop)   mode_stop ;;
    *)
        echo "Usage: ./run.sh [direct|nohup|screen|tmux|logs|stop]"
        echo ""
        echo "  direct  — run in foreground (default)"
        echo "  nohup   — run in background, log to $LOG_FILE"
        echo "  screen  — run in detached screen session"
        echo "  tmux    — run in detached tmux session"
        echo "  logs    — tail log file"
        echo "  stop    — kill running bot process"
        ;;
esac