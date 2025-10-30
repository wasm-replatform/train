# CARS Integration

This crate provides integration with the CARS (via ArcGIS) and MyWorkSites platform. 

It includes functionality for ingesting and processing traffic data from various sources, including
TomTom's traffic services.

## MyWorkSites API

The MyWorkSites API is a RESTful service that allows access to traffic management plans and related 
data. The API Documentation can be found [here](https://api.myworksites.co.nz/v1/prod/explorer/) but treat
with caution — it is not always correct.

### Sample queries

Worksite Search:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/worksite-search?filter={%22where%22%3A%20{%22worksiteCode%22%3A%20{%22eq%22%3A%20%22AT-W188834%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

All Worksites:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/worksites' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```


Traffic Management Plan Search:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/tmp-search?filter={%22where%22%3A%20{%22worksiteCode%22%3A%20{%22eq%22%3A%20%22AT-W188834%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

All TMPs:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/tmps' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

TMPs by ID:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/tmps/166603' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

All Deployments:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/deployments' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

Deployments by Layout ID:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/deployments?filter={%22where%22%3A%20{%22layoutId%22%3A%20{%22eq%22%3A%20%221375712%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/deployments?filter={%22where%22%3A%20{%22layoutId%22%3A%20{%22eq%22%3A%20%221375712%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

All Layouts:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/layouts' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

Layouts by TMP ID:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/layouts?filter={%22where%22%3A%20{%22tmpId%22%3A%20{%22eq%22%3A%20%22169957%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/layouts?filter={%22where%22%3A%20{%22tmpId%22%3A%20{%22eq%22%3A%20%22169957%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

Layout by ID:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/layouts/564547' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

Impact by ID:
```bash
curl --location 'https://api.myworksites.co.nz/v1/prod/impacts/6973464' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```

Impacts by Layout ID:
```bash
curl --location --globoff 'https://api.myworksites.co.nz/v1/prod/impacts?filter={%22where%22%3A%20{%22layoutId%22%3A%20{%22eq%22%3A%20%221375712%22}}}' \
--header 'Accept: application/json' \
--header 'x-api-key: <key>'
```