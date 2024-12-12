# Discuss
- sreport gibt auch total für Acc, nicht nur für alle user des accs. *relevant?*
  - https://slurm.schedmd.com/sreport.html#OPT_cluster-AccountUtilizationByUser
- slurm befehle nehmen timestamps in UTC oder Local?
- binary size wichtig? strip = kleine file size <-> keine backtraces

# NEXT
- update README
- (eventually) publish binaries on github (for ansible deploy)

## Data collection
- schauen ob der avg cpu/gpu über die Messzeit ist
    - nein, bei GPU ist über 1/6 - 1s, wir messen aber alle 30sec
- **NodeUsage renamen** (und anderen Stuff evtl auch)

# Features
- bei zu wenig GPU Auslastung: Anschreiben (und vlt leaderboard / hall of shame)
    - bei 100% pro Core auch (weil CPU boundness)
- GPU-Stunden
    - Slurm hat (hearsay) ne schnittstelle, die das aggregiert
## Data collection
- IO, file system usage%
- fds (z.B.)

- which user "illegaly" logged onto which ml node (none is allowed!)

### errors
- syslog entries (e.g.)

## API
### Data in memory
- could use RwLockRead/Write::downgrade_<…>() for increased performance (sharding without sharding so to speak)


# Old

## Bugs
- länge der felder auf max setzen (sacct)
    - längenangaben sind raus, sollte jetzt unbegrenzt sein (https://slurm.schedmd.com/squeue.html)
- tokio threads (max 8)
    - => #[tokio::main()] takes `worker_threads` as argument (setting 4 for now)

## Partial write
Somehow, one time the JSON was only partially written to disk.
### Possible solutions
- [x] Start a new JSON if anything happens.