# AT Dilax Apc Connector

# Description

<!-- Write project Description -->

### Table of Contents

-   [Verify that Works](#markdown-header-verify-that-works)
-   [Feature flags](#markdown-header-feature-flags)
-   [Changelog](#markdown-header-changelog)

# Build and run instructions

To run the project locally:

1. Copy and rename the `.env.example` to `.env`
2. Populate missing ENV variables in the `.env` (can be taken from `./azure/parameters.json` file and/or from an azure portal, LastPass, or other places the system needs the connection to)
3. Start the project with `npm run dev` command (to start the project from `src` itself). Alternatively, `npm run build && npm start` (to compile and start the project)

# Verify that Works

-   Check Papertrail Logs

```
Jul 10 13:33:56 at-realtime-dev www-dev-at-dilax-apc-connector-01 [6648] info: Starting DILAX APC connector
Jul 10 13:33:56 at-realtime-dev www-dev-at-dilax-apc-connector-01 [6648] info: Starting Kafka Producer
Jul 10 13:33:56 at-realtime-dev www-dev-at-dilax-apc-connector-01 [6648] info: DILAX APC connector started
```

-   Check NR metrics and lags
