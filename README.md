# Worldbuilder

Worldbuilder is a local-first worldbuilding studio. Each world is stored in a portable project folder:

```text
My World/
├── project.json
├── worldbuilder.sqlite
├── .gitignore
└── assets/
    ├── images/
    ├── videos/
    ├── maps/
    └── files/
```

Open and close project folders from the app. Files attached through the UI are copied into the project and recorded with a SHA-256 hash. Git can be initialized, inspected, and committed from the project menu; JSON snapshots are used internally for backups and recovery.
