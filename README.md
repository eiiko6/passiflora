# Passiflora server

## Configuration

- Specify the JWT secret with the `PASSIFLORA_JWT_SECRET` environment variable.
- Specify the server's port with the `PASSIFLORA_PORT` environment variable. Defaults to `8080`.
- To enable user registration, pass in `PASSIFLORA_ALLOW_REGISTRATION=true`.
- `PASSIFLORA_DATA_DIR=/var/lib/passiflora-server/` will set the location for the hosted files to `/var/lib/passiflora-server/`.

## Development

You can use this to setup the database:

```sh
podman build -t passiflora-db ./db
podman run --replace -d \
          --name passiflora-db \
          -p 5432:5432 \
          passiflora-db
```
