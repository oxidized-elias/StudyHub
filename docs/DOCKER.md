# Docker

StudyHub runs as one container with the same Node launcher and integrated native server as a manual installation. The image builds the native executable, bundles the logo/frontends, and includes Poppler, ImageMagick and HEIF conversion tools. Rust and npm installs are not needed on the host. An AI endpoint is configured separately; models are not included or downloaded.

## Start with Compose

Install Docker Engine and the Docker Compose plugin, then from the repository:

```bash
docker compose up -d --build
```

Open **http://localhost:8082**. The default published address is localhost. Two named volumes persist independently of the container:

| Mount | Contents |
| --- | --- |
| `/app/data` | Settings, provider keys, import originals, correction lessons, SQLite history and notebooks. |
| `/vault` | Approved Obsidian notes and subject-wide BigIdeas files. |

The image runs as a non-root user (UID/GID 1000 by default). Newly created named volumes inherit its directory ownership. There is no automatic deletion or migration of existing host data.

Optional settings:

```bash
cp .env.docker.example .env.docker
# Edit .env.docker, then:
docker compose --env-file .env.docker up -d --build
```

Continue passing the same env file for later Compose commands. Compose also reads a project `.env` by default; the supplied Compose file fixes internal bind/data/vault paths so a manual-install `.env` cannot accidentally substitute host paths inside the container. Explicit environment settings override image defaults; see [Docker's environment precedence](https://docs.docker.com/compose/how-tos/environment-variables/envvars-precedence/).

| Setting | Default | Meaning |
| --- | --- | --- |
| `STUDYHUB_HOST` | `127.0.0.1` | Host interface for the published port; use `0.0.0.0` for LAN access. |
| `STUDYHUB_PORT` | `8082` | Host port. |
| `STUDYHUB_URL` | `http://localhost:<port>` | Public origin; set the real HTTPS origin behind a reverse proxy. |
| `STUDYHUB_USER` | `admin` | Optional HTTP Basic username. |
| `STUDYHUB_PASSWORD` | empty | Enables application authentication when set. |
| `STUDYHUB_VAULT_PATH` | named volume | Optional absolute path to an existing vault on the host. |
| `STUDYHUB_UID`, `STUDYHUB_GID` | `1000` | Image build user/group IDs for matching host folder ownership. |

The application always listens on port 8082 **inside** the container, even when a different host port is published. The health check uses this internal port and supplies configured Basic credentials without putting them in its URL or command arguments. It requires both application modules to be initialized. See [Compose services](https://docs.docker.com/reference/compose-file/services/) for port, volume and service options.

## Use a host Obsidian vault

Create the folder on the Docker host and make it writable by the image user. Put its absolute path in `.env.docker`:

```dotenv
STUDYHUB_VAULT_PATH=/home/your-user/Documents/StudyHub_Vault
```

If the host account is not UID/GID 1000, set `STUDYHUB_UID` and `STUDYHUB_GID` to the output of `id -u` and `id -g` before building. Existing volume ownership does not change when rebuilding with different IDs; correct it separately if intentionally changing the service user.

In StudyHub's Settings select **`/vault`**. The folder browser runs inside the container and can only see mounted/container paths. `/vault` refers to the host folder configured above; entering the host's original absolute path in the UI does not mount it. New named-volume installs use `/vault` automatically. A saved vault preference takes precedence over the initial environment default.

## Connect models

Set provider/model URLs in the application, as with a manual install:

- **Remote Ollama:** use its reachable server address, such as `http://192.168.1.50:11434`.
- **Ollama on the Docker host:** Compose adds `host.docker.internal`; use `http://host.docker.internal:11434`. On Linux, Ollama must listen on an interface reachable from Docker's bridge. A server bound only to host `127.0.0.1` is not reachable through that gateway. Docker Desktop handles host access differently; see [host networking guidance](https://docs.docker.com/desktop/features/networking/#i-want-to-connect-from-a-container-to-a-service-on-the-host).
- **Cloud providers:** enter the API key/profile in Settings. Keys persist in the data volume.

The StudyHub container does not run model inference, so it does not require GPU passthrough. GPU placement is managed by your Ollama/model server. Browser libraries still load from pinned CDNs; local model inference does not make the browser assets offline.

## Plain Docker

```bash
docker build -t studyhub:local .
docker run -d --init --name studyhub \
  -p 127.0.0.1:8082:8082 \
  --add-host=host.docker.internal:host-gateway \
  -v studyhub_data:/app/data \
  -v studyhub_vault:/vault \
  --restart unless-stopped \
  studyhub:local
```

Both mounts are important: the vault is separate from other user data. Pass environment variables at runtime for authentication/public URLs. The image is built locally; no published registry image is assumed.

## Operations and backups

```bash
docker compose ps
docker compose logs -f --tail=100 studyhub
docker compose stop
docker compose up -d --build  # rebuild after updating the source
```

`docker compose down` removes the containers/network and preserves named volumes. **`docker compose down -v` deletes the named volumes and their user data.** It does not delete host bind-mounted vault files. Compose allows 45 seconds for shutdown; interrupted jobs remain retryable after restart.

Stop the service before taking consistent backups of **both** named volumes, or the data volume plus your host vault. Use `docker volume ls`/`docker volume inspect` to identify the project-prefixed volumes. Host-folder backups alone do not contain the notebook database when using the default named data volume.

To migrate a manual installation, stop both deployments first, back up the existing data/vault, and restore the data directory into the container's data volume. Mount or restore the existing vault at `/vault`, then set the saved vault location to `/vault` in Settings. Preserve the learning database and originals along with the notes database. Existing absolute recorded locations may refer to the old host; the notes resolver can recover using the configured vault when the relative note paths are preserved. No migration or overwrite is performed automatically by Compose.

## Verification

```bash
npm run test:docker-config
docker build -t studyhub:smoke .
node tests/docker-smoke.mjs studyhub:smoke
```

The smoke test uses disposable containers/volumes and no AI provider. It checks authenticated health, bundled logo, non-root execution, extraction commands, graceful shutdown, and notebook/vault persistence across container recreation, then removes only its test resources. CI builds and runs this test on a Docker-enabled runner. If the local Docker socket is inaccessible, Compose parsing and health-check logic can be tested, but the image build/runtime remains unverified until that test runs.
