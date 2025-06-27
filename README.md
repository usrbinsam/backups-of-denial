# Backups of Denial

This program makes a timestamped copy of your Save file each time the game writes to it.
The goal is to deny hackers the chance to corrupt your save file.

This runs as a separate process from the game; it is not a mod. You won't be banned for using it.

## Setup

- Create the config file: `C:\Users\YOUR USERNAME\.backups-of-denial.toml`

```toml
backup_dir = '''\path\for\backup\storage'''
save_game_dir = '''C:\Users\YOUR USERNAME\OneDrive\Documents\NBGI\DARK SOULS REMASTERED'''
```

- Start the executable: `backups-of-denial.exe`
- Start playing

