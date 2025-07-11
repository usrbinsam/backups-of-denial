# Backups of Denial: Dark Souls Realtime Save Game Backup

This program makes a timestamped copy of your Save file each time the game writes to it.
If a hacker invader manages to corrupt your game save, you have the opportunity to roll back to a good save file.

The goal here is to always have a fresh copy of a recent game state. The backup location will rapidly fill in
just a few hours of gameplay.

This runs as a separate process from the game; it is not a mod and therefore does not violate ToS.

Backups of Denial checks the basic structure of the save file after each backup. If it detects a corrupt file, it
stops creating backups to prevent copying over more corrupt files and subsequently pruning good ones.

## Setup

- Download from [Releases](https://github.com/usrbinsam/backups-of-denial/releases)
- Create the config file: `C:\Users\YOUR USERNAME\.backups-of-denial.toml`

```toml
backup_dir = '''\path\for\backup\storage'''
save_game_dir = '''C:\Users\YOUR USERNAME\OneDrive\Documents\NBGI\DARK SOULS REMASTERED'''
verify_bnd4 = true # verify backup file integrity
decryption_key = '0123456789ABCDEFFEDCBA9876543210' # Dark Souls Remastered key, only required if verify_bnd4 = true.
backup_mask = '*.sl2' # or '*.sl2.overhaul' if using PVP Overhaul Mod
retention_minutes = 180  # delete backups older than 3 hours
min_backup_count = 30 # keep a minimum of 30 backups before pruning files older than `retention_minutes`
```

- Start the executable: `backups-of-denial.exe`
    - :exclamation: You will likely get a "Windows SmartScreen" alert for the first run.
      Click **More Info** > **Run anyway**.
- Start playing

## Troubleshooting

If launching the executable does nothing, its probably crashing due to an invalid config, or 
a missing directory.

You can see the exit reason if you try launching from a command prompt.

## Help

Feel free to open a [GitHub Issue](https://github.com/usrbinsam/backups-of-denial/issues) if you need help or have a
feature suggestion.

## TODO

- Game Launcher: Wrapper around the Game executable so the watcher can automatically start and stop with the game.
- Sanity check on character data
- Multiple game support
