#!/usr/bin/env bash
# Sets up (or updates) the Learning Platform on a Debian/Ubuntu production server.
#
#   sudo ./setup.sh              first install: asks a few questions, then does everything
#   git pull && sudo ./setup.sh  update: reuses your answers, builds and deploys a new release
#   sudo ./setup.sh --reconfigure   ask the questions again
#
# What it does: installs system packages, Node.js, pnpm, Rust, PostgreSQL and Caddy; creates
# a service user, database and config; builds the app; deploys it as a versioned release
# behind Caddy with automatic HTTPS; adds a systemd service, firewall rules and daily database
# backups; verifies the deployment and rolls back automatically if the new release fails.
# Safe to run repeatedly: existing secrets, config and data are kept.

set -Eeuo pipefail
umask 022

APP=learning-platform
SERVICE_USER=learning-platform
DB_NAME=learning_platform
DB_USER=learning_platform
INSTALL_DIR=/opt/$APP
SRC_DIR=$INSTALL_DIR/src
RELEASES_DIR=$INSTALL_DIR/releases
CURRENT_LINK=$INSTALL_DIR/current
CONF_DIR=/etc/$APP
ENV_FILE=$CONF_DIR/$APP.env
SETUP_CONF=$CONF_DIR/setup.conf
BACKUP_DIR=/var/backups/$APP
LOG_FILE=/var/log/$APP-setup.log
CADDY_SITE=/etc/caddy/$APP.caddy
TRAFFIC_DIR=/var/lib/$APP-traffic
TRAFFIC_REPORT=$TRAFFIC_DIR/report.html
# Also stated on the /privacy page of the web app.
ACCESS_LOG_RETENTION_DAYS=30
export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo
export PATH="$CARGO_HOME/bin:$PATH"
export DEBIAN_FRONTEND=noninteractive
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0

MIN_NODE=22.17
NODE_MAJOR=24
MIN_RUST=1.88
MIN_POSTGRES=14
KEEP_RELEASES=3
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ASSUME_YES=false
RECONFIGURE=false

# ---------------------------------------------------------------------------- output

if [[ -t 1 ]]; then
	BOLD=$'\e[1m' DIM=$'\e[2m' RED=$'\e[31m' GREEN=$'\e[32m' YELLOW=$'\e[33m' BLUE=$'\e[34m' RESET=$'\e[0m'
else
	BOLD='' DIM='' RED='' GREEN='' YELLOW='' BLUE='' RESET=''
fi

step() { printf '\n%s==> %s%s\n' "$BOLD$BLUE" "$*" "$RESET"; }
info() { printf '    %s\n' "$*"; }
ok() { printf '    %s✓%s %s\n' "$GREEN" "$RESET" "$*"; }
warn() { printf '    %s!%s %s\n' "$YELLOW" "$RESET" "$*" >&2; }
die() {
	printf '\n%sError:%s %s\n' "$RED$BOLD" "$RESET" "$*" >&2
	exit 1
}

on_error() {
	local code=$? line=$1
	printf '\n%sSetup failed%s (exit %s, line %s). Full log: %s\n' "$RED$BOLD" "$RESET" "$code" "$line" "$LOG_FILE" >&2
	printf 'Fix the problem and run the script again; finished steps are skipped or repeated safely.\n' >&2
}
trap 'on_error $LINENO' ERR

usage() {
	cat <<EOF
Usage: sudo ./setup.sh [options]

Options:
  -y, --yes          Non-interactive: use saved answers, environment variables or defaults.
                     Variables: DOMAIN, ADMIN_EMAIL, ENABLE_FIREWALL, ENABLE_BACKUPS,
                     BACKUP_RETENTION_DAYS, ENABLE_AUTO_UPDATES, ENABLE_TRAFFIC_REPORT, APP_PORT.
      --reconfigure  Ask the setup questions again (config and secrets are kept).
  -h, --help         Show this help.
EOF
}

# ---------------------------------------------------------------------------- prompts

# ask VAR "Question" "default"
ask() {
	local var=$1 question=$2 default=${3-} answer
	if $ASSUME_YES; then
		printf -v "$var" '%s' "${!var:-$default}"
		return
	fi
	local shown=${!var:-$default}
	read -r -p "    $question${shown:+ [$shown]}: " answer </dev/tty || true
	printf -v "$var" '%s' "${answer:-$shown}"
}

# confirm "Question" y|n  -> exit status
confirm() {
	local question=$1 default=${2:-y} answer
	if $ASSUME_YES; then
		[[ $default == y ]]
		return
	fi
	local hint='[Y/n]'
	[[ $default == y ]] || hint='[y/N]'
	read -r -p "    $question $hint " answer </dev/tty || true
	answer=${answer:-$default}
	[[ ${answer,,} == y* ]]
}

pause() {
	$ASSUME_YES && return
	read -r -p "    ${1:-Press Enter to continue...} " _ </dev/tty || true
}

# ---------------------------------------------------------------------------- helpers

version_ge() { [[ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" == "$2" ]]; }

random_secret() { openssl rand -hex 24; }

env_value() { # env_value KEY -> value from $ENV_FILE
	[[ -f $ENV_FILE ]] || return 0
	sed -n "s/^$1=//p" "$ENV_FILE" | tail -n1
}

set_env_value() { # set_env_value KEY VALUE (adds or replaces)
	local key=$1 value=$2
	if grep -q "^$key=" "$ENV_FILE"; then
		local escaped
		escaped=$(printf '%s' "$value" | sed 's/[&|\\]/\\&/g')
		sed -i "s|^$key=.*|$key=$escaped|" "$ENV_FILE"
	else
		printf '%s=%s\n' "$key" "$value" >>"$ENV_FILE"
	fi
}

ensure_env_value() { # adds KEY only if missing, so manual edits survive updates
	grep -q "^$1=" "$ENV_FILE" || {
		printf '%s=%s\n' "$1" "$2" >>"$ENV_FILE"
		info "Added new setting $1"
	}
}

apt_install() {
	local missing=()
	for package in "$@"; do
		dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q 'install ok installed' || missing+=("$package")
	done
	((${#missing[@]})) || return 0
	info "Installing ${missing[*]}"
	apt-get install -y -qq --no-install-recommends "${missing[@]}" >/dev/null
}

public_ip() {
	curl -fsS --max-time 5 https://api.ipify.org 2>/dev/null || hostname -I 2>/dev/null | awk '{print $1}'
}

port_owner() { ss -ltnpH "sport = :$1" 2>/dev/null | sed -n 's/.*users:((\"\([^"]*\)".*/\1/p' | head -n1; }

as_service_user() { # runs the app binary of a release with the production environment
	local release=$1
	shift
	(
		set -a
		# shellcheck disable=SC1090
		. "$ENV_FILE"
		set +a
		export COURSES_DIR="$release/courses" WEB_DIST_DIR="$release/web"
		cd "$release"
		runuser -u "$SERVICE_USER" -- "$release/bin/$APP" "$@"
	)
}

wait_for() { # wait_for URL SECONDS
	local url=$1 timeout=$2 elapsed=0
	until curl -fsS --max-time 5 "$url" 2>/dev/null | grep -q '"database":"ok"'; do
		((elapsed >= timeout)) && return 1
		sleep 2
		elapsed=$((elapsed + 2))
	done
}

# ---------------------------------------------------------------------------- steps

preflight() {
	step "Checking the server"
	[[ $EUID -eq 0 ]] || die "Run this script as root: sudo ./setup.sh"
	[[ -f $REPO_DIR/Cargo.toml && -d $REPO_DIR/courses && -d $REPO_DIR/apps/web ]] ||
		die "Run setup.sh from the root of the learning-platform repository."

	# shellcheck disable=SC1091
	. /etc/os-release
	case "${ID:-}:${ID_LIKE:-}" in
	debian:* | ubuntu:* | *:*debian*) ;;
	*) die "Only Debian and Ubuntu are supported (found ${PRETTY_NAME:-unknown})." ;;
	esac
	if [[ $ID == ubuntu ]] && ! version_ge "${VERSION_ID:-0}" 22.04; then die "Ubuntu 22.04 or newer is required."; fi
	if [[ $ID == debian ]] && ! version_ge "${VERSION_ID:-0}" 12; then die "Debian 12 or newer is required."; fi
	ok "${PRETTY_NAME}, $(uname -m)"

	[[ -d /run/systemd/system ]] || die "systemd is required to run the service."
	command -v apt-get >/dev/null || die "apt-get not found."

	local mem_mb disk_gb
	mem_mb=$(awk '/MemTotal/ {print int($2 / 1024)}' /proc/meminfo)
	disk_gb=$(df -Pk /opt | awk 'NR == 2 {print int($4 / 1024 / 1024)}')
	ok "${mem_mb} MB memory, ${disk_gb} GB free disk"
	((disk_gb >= 6)) || die "At least 6 GB of free disk space is needed to build (found ${disk_gb} GB)."
	MEM_MB=$mem_mb
}

load_or_ask_settings() {
	step "Settings"
	if [[ -f $SETUP_CONF ]] && ! $RECONFIGURE; then
		# shellcheck disable=SC1090
		. "$SETUP_CONF"
		ENABLE_TRAFFIC_REPORT=${ENABLE_TRAFFIC_REPORT:-yes}
		ok "Using saved answers from $SETUP_CONF (run with --reconfigure to change them)"
		return
	fi

	cat <<EOF
    A domain name gives you HTTPS with an automatic Let's Encrypt certificate. Point its
    DNS A/AAAA record at this server first. Without a domain the platform runs on plain HTTP
    on the server's IP address. That works for a quick test, but agents can't sign in over
    MCP (OAuth needs HTTPS) and browsers treat the site as insecure.

EOF
	ask DOMAIN "Domain name (empty for HTTP on the IP address)" ""
	DOMAIN=${DOMAIN,,}
	DOMAIN=${DOMAIN#http://}
	DOMAIN=${DOMAIN#https://}
	DOMAIN=${DOMAIN%%/*}
	if [[ -n $DOMAIN && ! $DOMAIN =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$ ]]; then
		die "\"$DOMAIN\" is not a valid domain name."
	fi

	info "The admin email receives certificate expiry notices and becomes super admin once that"
	info "account is registered on the platform."
	while :; do
		ask ADMIN_EMAIL "Admin email" ""
		[[ $ADMIN_EMAIL =~ ^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$ ]] && break
		$ASSUME_YES && die "Set ADMIN_EMAIL to a valid email address."
		warn "Enter a valid email address."
	done
	ADMIN_EMAIL=${ADMIN_EMAIL,,}

	ask APP_PORT "Internal port for the app (only reachable through Caddy)" "8080"
	if ! [[ $APP_PORT =~ ^[0-9]+$ ]] || ((APP_PORT < 1024 || APP_PORT > 65535)); then
		die "Port must be between 1024 and 65535."
	fi

	ENABLE_FIREWALL=${ENABLE_FIREWALL:-}
	if [[ -z $ENABLE_FIREWALL ]]; then
		confirm "Configure the ufw firewall (allow SSH, HTTP and HTTPS only)?" y && ENABLE_FIREWALL=yes || ENABLE_FIREWALL=no
	fi
	ENABLE_BACKUPS=${ENABLE_BACKUPS:-}
	if [[ -z $ENABLE_BACKUPS ]]; then
		confirm "Make a daily database backup to $BACKUP_DIR?" y && ENABLE_BACKUPS=yes || ENABLE_BACKUPS=no
	fi
	if [[ $ENABLE_BACKUPS == yes ]]; then
		ask BACKUP_RETENTION_DAYS "Days to keep backups" "14"
		[[ $BACKUP_RETENTION_DAYS =~ ^[0-9]+$ ]] || die "Retention must be a number of days."
	fi
	ENABLE_AUTO_UPDATES=${ENABLE_AUTO_UPDATES:-}
	if [[ -z $ENABLE_AUTO_UPDATES ]]; then
		confirm "Install security updates automatically (unattended-upgrades)?" y && ENABLE_AUTO_UPDATES=yes || ENABLE_AUTO_UPDATES=no
	fi
	ENABLE_TRAFFIC_REPORT=${ENABLE_TRAFFIC_REPORT:-}
	if [[ -z $ENABLE_TRAFFIC_REPORT ]]; then
		confirm "Show traffic statistics to super admins (built from the web server log)?" y && ENABLE_TRAFFIC_REPORT=yes || ENABLE_TRAFFIC_REPORT=no
	fi

	install -d -m 750 "$CONF_DIR"
	cat >"$SETUP_CONF" <<EOF
# Answers for setup.sh. Edit them, or run setup.sh --reconfigure.
DOMAIN=$DOMAIN
ADMIN_EMAIL=$ADMIN_EMAIL
APP_PORT=$APP_PORT
ENABLE_FIREWALL=$ENABLE_FIREWALL
ENABLE_BACKUPS=$ENABLE_BACKUPS
BACKUP_RETENTION_DAYS=${BACKUP_RETENTION_DAYS:-14}
ENABLE_AUTO_UPDATES=$ENABLE_AUTO_UPDATES
ENABLE_TRAFFIC_REPORT=$ENABLE_TRAFFIC_REPORT
EOF
	chmod 600 "$SETUP_CONF"
	ok "Saved answers to $SETUP_CONF"
}

resolve_public_url() {
	if [[ -n $DOMAIN ]]; then
		PUBLIC_URL="https://$DOMAIN"
		PUBLIC_HOST=$DOMAIN
	else
		PUBLIC_HOST=$(public_ip)
		[[ -n $PUBLIC_HOST ]] || die "Could not determine this server's IP address; set a DOMAIN instead."
		PUBLIC_URL="http://$PUBLIC_HOST"
	fi
}

ensure_swap() {
	step "Memory for building"
	local swap_mb
	swap_mb=$(awk '/SwapTotal/ {print int($2 / 1024)}' /proc/meminfo)
	if ((MEM_MB + swap_mb >= 4000)); then
		ok "Enough memory to compile (${MEM_MB} MB RAM, ${swap_mb} MB swap)"
		return
	fi
	warn "Compiling the Rust release build needs about 4 GB of memory; this server has ${MEM_MB} MB RAM and ${swap_mb} MB swap."
	if [[ -f /swapfile ]]; then
		warn "/swapfile exists but is not enough; the build may be slow or run out of memory."
		return
	fi
	if confirm "Create a 4 GB swap file at /swapfile?" y; then
		fallocate -l 4G /swapfile 2>/dev/null || dd if=/dev/zero of=/swapfile bs=1M count=4096 status=none
		chmod 600 /swapfile
		mkswap /swapfile >/dev/null
		swapon /swapfile
		grep -q '^/swapfile ' /etc/fstab || echo '/swapfile none swap sw 0 0' >>/etc/fstab
		ok "4 GB swap enabled"
	fi
}

install_system_packages() {
	step "Installing system packages"
	apt-get update -qq
	apt_install ca-certificates curl gnupg git rsync openssl build-essential pkg-config \
		postgresql postgresql-contrib debian-keyring debian-archive-keyring apt-transport-https
	[[ $ENABLE_FIREWALL == yes ]] && apt_install ufw
	[[ $ENABLE_AUTO_UPDATES == yes ]] && apt_install unattended-upgrades

	local pg_major
	pg_major=$(find /usr/lib/postgresql -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort -V | tail -n1)
	[[ $pg_major =~ ^[0-9]+$ ]] || die "Could not find the installed PostgreSQL version."
	((pg_major >= MIN_POSTGRES)) || die "PostgreSQL $MIN_POSTGRES or newer is required (found $pg_major)."
	systemctl enable --now postgresql >/dev/null 2>&1
	ok "PostgreSQL $pg_major running"
}

install_node() {
	step "Installing Node.js and pnpm"
	local current=""
	command -v node >/dev/null && current=$(node --version | tr -d v)
	if [[ -z $current ]] || ! version_ge "$current" "$MIN_NODE"; then
		info "Installing Node.js $NODE_MAJOR from NodeSource (found: ${current:-none})"
		install -d -m 755 /etc/apt/keyrings
		curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key |
			gpg --dearmor --yes -o /etc/apt/keyrings/nodesource.gpg
		echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_$NODE_MAJOR.x nodistro main" \
			>/etc/apt/sources.list.d/nodesource.list
		apt-get update -qq
		apt-get install -y -qq nodejs >/dev/null
	fi
	ok "Node.js $(node --version)"
	# Newer Node.js releases no longer bundle corepack.
	command -v corepack >/dev/null || npm install -g --silent corepack >/dev/null
	corepack enable
	(cd "$REPO_DIR" && corepack install >/dev/null)
	ok "pnpm $(cd "$REPO_DIR" && pnpm --version)"
}

install_rust() {
	step "Installing Rust"
	if [[ ! -x $CARGO_HOME/bin/rustup ]]; then
		info "Installing rustup to $CARGO_HOME"
		curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path >/dev/null
	else
		rustup update stable --no-self-update >/dev/null 2>&1 || warn "Could not update Rust; using the installed toolchain."
	fi
	rustup default stable >/dev/null
	local version
	version=$(rustc --version | awk '{print $2}')
	version_ge "$version" "$MIN_RUST" || die "Rust $MIN_RUST or newer is required (found $version)."
	ok "Rust $version"
}

install_caddy() {
	step "Installing Caddy (reverse proxy with automatic HTTPS)"
	if ! command -v caddy >/dev/null; then
		for port in 80 443; do
			local owner
			owner=$(port_owner "$port")
			if [[ -n $owner && $owner != caddy ]]; then
				warn "Port $port is used by \"$owner\". Caddy needs ports 80 and 443."
				pause "Stop or reconfigure $owner in another terminal, then press Enter..."
			fi
		done
		curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/gpg.key |
			gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
		curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt \
			>/etc/apt/sources.list.d/caddy-stable.list
		apt-get update -qq
		apt-get install -y -qq caddy >/dev/null
	fi
	ok "$(caddy version | awk '{print "Caddy " $1}')"
}

create_service_user() {
	step "Creating the service user"
	if ! id "$SERVICE_USER" >/dev/null 2>&1; then
		useradd --system --home-dir "$INSTALL_DIR" --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
	fi
	install -d -m 755 "$INSTALL_DIR" "$RELEASES_DIR"
	install -d -m 750 -g "$SERVICE_USER" "$CONF_DIR"
	ok "User $SERVICE_USER, files in $INSTALL_DIR"
}

setup_database() {
	step "Setting up the database"
	local url password=""
	url=$(env_value DATABASE_URL)
	if [[ $url =~ ^postgres(ql)?://$DB_USER:([^@]+)@ ]]; then
		password=${BASH_REMATCH[2]}
	fi
	[[ -n $password ]] || password=$(random_secret)
	[[ $password =~ ^[A-Za-z0-9]+$ ]] || die "The database password in $ENV_FILE must be letters and digits only."

	runuser -u postgres -- psql -q -v ON_ERROR_STOP=1 <<SQL
DO \$\$
BEGIN
	IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '$DB_USER') THEN
		CREATE ROLE $DB_USER LOGIN PASSWORD '$password';
	ELSE
		ALTER ROLE $DB_USER WITH LOGIN PASSWORD '$password';
	END IF;
END
\$\$;
SQL
	if ! runuser -u postgres -- psql -tAc "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1; then
		runuser -u postgres -- createdb --owner="$DB_USER" "$DB_NAME"
	fi
	DATABASE_URL="postgres://$DB_USER:$password@localhost:5432/$DB_NAME"
	PGPASSWORD=$password psql -h localhost -U "$DB_USER" -d "$DB_NAME" -tAc 'SELECT 1' >/dev/null ||
		die "Cannot log in to PostgreSQL as $DB_USER over localhost; check pg_hba.conf allows password logins."
	ok "Database $DB_NAME owned by $DB_USER (password stored in $ENV_FILE only)"
}

write_config() {
	step "Writing the configuration"
	local cookie_secure=false hosts="localhost,127.0.0.1,::1"
	local traffic_setting="# The traffic report for super admins is disabled (setup.sh --reconfigure enables it)."
	[[ -n $DOMAIN ]] && cookie_secure=true
	if [[ $ENABLE_TRAFFIC_REPORT == yes ]]; then
		traffic_setting="# HTML traffic report, rebuilt from the Caddy access log every 15 minutes.
TRAFFIC_REPORT_PATH=$TRAFFIC_REPORT"
	fi
	hosts="$PUBLIC_HOST,$hosts"

	if [[ ! -f $ENV_FILE ]]; then
		cat >"$ENV_FILE" <<EOF
# Production configuration for the Learning Platform, read by the systemd service.
# Apply changes with: sudo systemctl restart $APP
# Re-running setup.sh keeps this file and only adds settings that are missing.

DATABASE_URL=$DATABASE_URL
DATABASE_MAX_CONNECTIONS=10

# Only Caddy talks to the app; it is not reachable from outside.
API_BIND_ADDRESS=127.0.0.1:$APP_PORT
# Public address of the platform. Used in agent setup instructions and OAuth metadata.
PUBLIC_BASE_URL=$PUBLIC_URL
COURSES_DIR=$CURRENT_LINK/courses
WEB_DIST_DIR=$CURRENT_LINK/web

# Hosts accepted on the MCP endpoint (DNS-rebinding protection).
MCP_ALLOWED_HOSTS=$hosts
# Caddy forwards the client address; used for rate limiting.
TRUST_PROXY_HEADERS=true

# Accounts with these emails become super admin (course studio) at startup.
SUPER_ADMIN_EMAILS=$ADMIN_EMAIL

$traffic_setting

COOKIE_SECURE=$cookie_secure
# Published course versions are immutable: bump "version" when you change a course file.
MUTABLE_COURSE_VERSIONS=false

RUST_LOG=info,lp_api=info,sqlx=warn,tower_http=warn
EOF
		ok "Created $ENV_FILE"
	else
		set_env_value DATABASE_URL "$DATABASE_URL"
		ensure_env_value DATABASE_MAX_CONNECTIONS 10
		ensure_env_value API_BIND_ADDRESS "127.0.0.1:$APP_PORT"
		ensure_env_value PUBLIC_BASE_URL "$PUBLIC_URL"
		ensure_env_value COURSES_DIR "$CURRENT_LINK/courses"
		ensure_env_value WEB_DIST_DIR "$CURRENT_LINK/web"
		ensure_env_value MCP_ALLOWED_HOSTS "$hosts"
		ensure_env_value TRUST_PROXY_HEADERS true
		ensure_env_value SUPER_ADMIN_EMAILS "$ADMIN_EMAIL"
		ensure_env_value COOKIE_SECURE "$cookie_secure"
		ensure_env_value MUTABLE_COURSE_VERSIONS false
		if [[ $ENABLE_TRAFFIC_REPORT == yes ]]; then ensure_env_value TRAFFIC_REPORT_PATH "$TRAFFIC_REPORT"; fi
		ensure_env_value RUST_LOG "info,lp_api=info,sqlx=warn,tower_http=warn"
		if $RECONFIGURE; then
			set_env_value PUBLIC_BASE_URL "$PUBLIC_URL"
			set_env_value MCP_ALLOWED_HOSTS "$hosts"
			set_env_value COOKIE_SECURE "$cookie_secure"
			set_env_value API_BIND_ADDRESS "127.0.0.1:$APP_PORT"
		fi
		ok "Kept existing $ENV_FILE"
	fi
	chown root:"$SERVICE_USER" "$ENV_FILE"
	chmod 640 "$ENV_FILE"

	if ! $ASSUME_YES; then
		printf '\n%s%s%s\n' "$DIM" "$(sed -E 's/(DATABASE_URL=postgres:\/\/[^:]+:)[^@]+@/\1••••••@/' "$ENV_FILE")" "$RESET"
		echo
		info "Review the configuration above. Common edits: SUPER_ADMIN_EMAILS (comma-separated),"
		info "DATABASE_MAX_CONNECTIONS, RUST_LOG."
		if confirm "Open it in an editor now?" n; then
			"${EDITOR:-nano}" "$ENV_FILE" </dev/tty >/dev/tty
		fi
	fi
	validate_config
}

validate_config() {
	local key missing=()
	for key in DATABASE_URL API_BIND_ADDRESS PUBLIC_BASE_URL COURSES_DIR WEB_DIST_DIR MCP_ALLOWED_HOSTS; do
		[[ -n $(env_value "$key") ]] || missing+=("$key")
	done
	((${#missing[@]} == 0)) || die "Missing settings in $ENV_FILE: ${missing[*]}"
	[[ $(env_value PUBLIC_BASE_URL) =~ ^https?://[^/]+$ ]] || die "PUBLIC_BASE_URL must look like https://example.com (no trailing slash or path)."
	if [[ $(env_value COOKIE_SECURE) == true && $(env_value PUBLIC_BASE_URL) == http://* ]]; then
		die "COOKIE_SECURE=true needs an https:// PUBLIC_BASE_URL, otherwise nobody can sign in."
	fi
	APP_ADDRESS=$(env_value API_BIND_ADDRESS)
	ok "Configuration is valid"
}

build_release() {
	step "Building the application (the first build takes 5–15 minutes)"
	install -d -m 755 "$SRC_DIR"
	rsync -a --delete \
		--exclude '/target/' --exclude 'node_modules/' --exclude '/apps/web/build/' \
		--exclude '/apps/web/.svelte-kit/' --exclude '.env' --exclude '.env.*' --exclude '.git/' \
		"$REPO_DIR"/ "$SRC_DIR"/
	cd "$SRC_DIR"

	info "Installing JavaScript dependencies"
	pnpm install --frozen-lockfile --filter web... --reporter=silent
	info "Building the web app"
	pnpm --filter web build >/dev/null
	[[ -f apps/web/build/index.html ]] || die "The web build did not produce apps/web/build/index.html."

	local jobs=$((MEM_MB / 1500))
	((jobs < 1)) && jobs=1
	((jobs > $(nproc))) && jobs=$(nproc)
	info "Compiling the server ($jobs parallel jobs)"
	CARGO_BUILD_JOBS=$jobs cargo build --release --locked --quiet -p lp-api --bin "$APP"

	RELEASE_ID=$(date +%Y%m%d%H%M%S)
	local sha
	sha=$(git -C "$REPO_DIR" rev-parse --short HEAD 2>/dev/null || true)
	[[ -n $sha ]] && RELEASE_ID="$RELEASE_ID-$sha"
	RELEASE_DIR=$RELEASES_DIR/$RELEASE_ID
	install -d -m 755 "$RELEASE_DIR/bin"
	install -m 755 "target/release/$APP" "$RELEASE_DIR/bin/$APP"
	cp -a apps/web/build "$RELEASE_DIR/web"
	cp -a courses "$RELEASE_DIR/courses"
	chown -R root:root "$RELEASE_DIR"
	cd /
	ok "Release $RELEASE_ID"

	info "Validating course files"
	as_service_user "$RELEASE_DIR" check-courses >/dev/null || die "Course validation failed (see output above)."
	info "Applying database migrations"
	as_service_user "$RELEASE_DIR" migrate >/dev/null
	ok "Courses valid, database up to date"
}

install_service() {
	step "Installing the systemd service"
	cat >/etc/systemd/system/$APP.service <<EOF
[Unit]
Description=Learning Platform
Documentation=file://$CURRENT_LINK
After=network-online.target postgresql.service
Wants=network-online.target postgresql.service

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
EnvironmentFile=$ENV_FILE
WorkingDirectory=$CURRENT_LINK
ExecStart=$CURRENT_LINK/bin/$APP serve
Restart=on-failure
RestartSec=3
TimeoutStopSec=20
LimitNOFILE=65536

# Hardening: the app only needs to read its release and talk to PostgreSQL over TCP.
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectKernelLogs=true
ProtectControlGroups=true
ProtectClock=true
ProtectHostname=true
RestrictNamespaces=true
RestrictRealtime=true
RestrictSUIDSGID=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
CapabilityBoundingSet=
SystemCallArchitectures=native
SystemCallFilter=@system-service
UMask=0077

[Install]
WantedBy=multi-user.target
EOF
	systemctl daemon-reload
	systemctl enable "$APP" >/dev/null 2>&1

	# Admin commands with the production config, e.g. `sudo learning-platform grant-super-admin a@b.c`.
	cat >/usr/local/bin/$APP <<EOF
#!/usr/bin/env bash
set -euo pipefail
[[ \$EUID -eq 0 ]] || exec sudo "\$0" "\$@"
set -a
. $ENV_FILE
set +a
cd $CURRENT_LINK
exec runuser -u $SERVICE_USER -- $CURRENT_LINK/bin/$APP "\$@"
EOF
	chmod 755 /usr/local/bin/$APP
	ok "$APP.service and the \`$APP\` admin command"
}

activate_release() {
	step "Starting the new release"
	local previous=""
	[[ -L $CURRENT_LINK ]] && previous=$(readlink -f "$CURRENT_LINK")
	ln -sfn "$RELEASE_DIR" "$CURRENT_LINK.new"
	mv -T "$CURRENT_LINK.new" "$CURRENT_LINK"
	systemctl restart "$APP"

	if wait_for "http://$APP_ADDRESS/api/health" 60; then
		ok "The app is healthy on $APP_ADDRESS"
	else
		warn "The new release did not become healthy. Recent logs:"
		journalctl -u "$APP" -n 40 --no-pager >&2 || true
		if [[ -n $previous && -d $previous ]]; then
			warn "Rolling back to $(basename "$previous")"
			ln -sfn "$previous" "$CURRENT_LINK.new"
			mv -T "$CURRENT_LINK.new" "$CURRENT_LINK"
			systemctl restart "$APP"
		fi
		die "Deployment failed; see the logs above (journalctl -u $APP)."
	fi

	# Old releases allow a quick manual rollback; keep a few.
	local old
	# Release names start with a timestamp, so name order is age order.
	mapfile -t old < <(find "$RELEASES_DIR" -mindepth 1 -maxdepth 1 -type d -printf '%p\n' | sort -r | tail -n +$((KEEP_RELEASES + 1)))
	for dir in "${old[@]}"; do
		[[ $(readlink -f "$dir") == $(readlink -f "$CURRENT_LINK") ]] || rm -rf "$dir"
	done
}

check_dns() {
	[[ -n $DOMAIN ]] || return 0
	local server resolved
	server=$(public_ip)
	resolved=$(getent ahosts "$DOMAIN" | awk '{print $1}' | sort -u | tr '\n' ' ')
	if [[ -z $resolved ]]; then
		warn "$DOMAIN does not resolve yet. Let's Encrypt can only issue a certificate once it does."
		pause "Create a DNS A record for $DOMAIN pointing to ${server:-this server}, then press Enter..."
	elif [[ -n $server && " $resolved " != *" $server "* ]]; then
		warn "$DOMAIN resolves to $resolved but this server's public IP is $server."
		warn "Ignore this if the server sits behind a proxy or NAT that forwards ports 80 and 443."
		pause "Fix the DNS record if needed, then press Enter..."
	else
		ok "$DOMAIN points to this server ($server)"
	fi
}

# Access logs hold visitor IP addresses, so they are deleted after a fixed number of days.
configure_log_rotation() {
	cat >/etc/logrotate.d/$APP <<EOF
/var/log/caddy/$APP.log {
	daily
	rotate $ACCESS_LOG_RETENTION_DAYS
	compress
	delaycompress
	missingok
	notifempty
	copytruncate
	su caddy caddy
}
EOF
}

configure_caddy() {
	step "Configuring Caddy"
	check_dns
	local site_address=":80" tls_line="" hsts=""
	if [[ -n $DOMAIN ]]; then
		site_address=$DOMAIN
		tls_line="	tls $ADMIN_EMAIL"
		hsts='		Strict-Transport-Security "max-age=31536000"'
	fi
	install -d -m 755 /var/log/caddy
	chown caddy:caddy /var/log/caddy 2>/dev/null || true
	cat >"$CADDY_SITE" <<EOF
# Managed by $APP setup.sh; changes are overwritten on the next run.
$site_address {
$tls_line
	# The app compresses responses itself and streams server-sent events, so no
	# encode directive here and no buffering.
	reverse_proxy $APP_ADDRESS {
		flush_interval -1
	}
	header {
$hsts
		-Server
	}
	log {
		output file /var/log/caddy/$APP.log {
			roll_disabled
		}
	}
}
EOF
	local caddyfile=/etc/caddy/Caddyfile
	if [[ ! -f $caddyfile ]] || grep -q '/usr/share/caddy' "$caddyfile"; then
		# The package's welcome page; replace it.
		printf '# Sites are kept in separate files.\nimport %s\n' "$CADDY_SITE" >"$caddyfile"
	elif ! grep -qF "import $CADDY_SITE" "$caddyfile"; then
		printf '\nimport %s\n' "$CADDY_SITE" >>"$caddyfile"
		info "Added the site to your existing $caddyfile"
	fi
	caddy validate --config "$caddyfile" --adapter caddyfile >/dev/null 2>&1 ||
		die "Caddy rejected the configuration; run: caddy validate --config $caddyfile"
	# Validating as root creates the log file owned by root, which the caddy service can't open.
	chown -R caddy:caddy /var/log/caddy
	configure_log_rotation
	systemctl enable caddy >/dev/null 2>&1
	systemctl reload-or-restart caddy
	ok "Caddy serves $PUBLIC_URL"
}

configure_firewall() {
	[[ $ENABLE_FIREWALL == yes ]] || return 0
	step "Configuring the firewall"
	local ports
	ports=$(sshd -T 2>/dev/null | awk '/^port / {print $2}' | sort -u)
	[[ -n $ports ]] || ports=22
	for port in $ports; do ufw allow "$port/tcp" comment 'SSH' >/dev/null; done
	ufw allow 80/tcp comment 'HTTP' >/dev/null
	ufw allow 443/tcp comment 'HTTPS' >/dev/null
	if ufw status | grep -q 'Status: active'; then
		ok "Rules added to the active firewall (SSH on ${ports//$'\n'/, }, 80, 443)"
	else
		warn "SSH stays allowed on port(s) ${ports//$'\n'/, }. Make sure that's the port you're connected on."
		if confirm "Enable the firewall now?" y; then
			ufw --force enable >/dev/null
			ok "Firewall enabled: SSH, HTTP and HTTPS allowed"
		fi
	fi
}

configure_backups() {
	[[ $ENABLE_BACKUPS == yes ]] || return 0
	step "Configuring daily database backups"
	install -d -m 700 -o postgres -g postgres "$BACKUP_DIR"
	cat >/usr/local/sbin/$APP-backup <<EOF
#!/usr/bin/env bash
# Daily PostgreSQL dump of the Learning Platform. Restore with:
#   sudo -u postgres pg_restore --clean --if-exists -d $DB_NAME <file>
set -euo pipefail
file=$BACKUP_DIR/$DB_NAME-\$(date +%Y%m%d-%H%M%S).dump
runuser -u postgres -- pg_dump --format=custom --file="\$file.partial" $DB_NAME
mv "\$file.partial" "\$file"
find $BACKUP_DIR -name '$DB_NAME-*.dump' -mtime +${BACKUP_RETENTION_DAYS:-14} -delete
EOF
	chmod 750 /usr/local/sbin/$APP-backup
	cat >/etc/systemd/system/$APP-backup.service <<EOF
[Unit]
Description=Back up the Learning Platform database
After=postgresql.service

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/$APP-backup
EOF
	cat >/etc/systemd/system/$APP-backup.timer <<EOF
[Unit]
Description=Daily Learning Platform database backup

[Timer]
OnCalendar=*-*-* 03:15
RandomizedDelaySec=30m
Persistent=true

[Install]
WantedBy=timers.target
EOF
	systemctl daemon-reload
	systemctl enable --now $APP-backup.timer >/dev/null 2>&1
	systemctl start $APP-backup.service
	ok "Daily at 03:15 to $BACKUP_DIR, kept ${BACKUP_RETENTION_DAYS:-14} days (first backup made now)"
}

configure_traffic_report() {
	[[ $ENABLE_TRAFFIC_REPORT == yes ]] || return 0
	step "Configuring the traffic report"
	apt_install goaccess
	install -d -m 750 -g "$SERVICE_USER" "$TRAFFIC_DIR"
	cat >/usr/local/sbin/$APP-traffic-report <<EOF
#!/usr/bin/env bash
# Builds the HTML report that super admins see at $PUBLIC_URL/admin/traffic.
set -euo pipefail
LOG_GLOB='/var/log/caddy/$APP.log*'
REPORT=$TRAFFIC_REPORT
GROUP=$SERVICE_USER
EOF
	# GoAccess 1.5 reads Caddy's address from remote_addr and newer versions from client_ip, so the
	# format is spelled out. IPs are shortened and query strings dropped (they can hold OAuth codes).
	cat >>/usr/local/sbin/$APP-traffic-report <<'EOF'
shopt -s nullglob
# shellcheck disable=SC2206
logs=($LOG_GLOB)
((${#logs[@]})) || exit 0
tmp="$(dirname "$REPORT")/.report.tmp.html"
zcat -f "${logs[@]}" | goaccess - \
	--log-format='{ "ts": "%x.%^", "request": { "client_ip": "%h", "proto": "%H", "method": "%m", "host": "%v", "uri": "%U", "headers": { "User-Agent": ["%u"] } }, "duration": "%T", "size": "%b", "status": "%s" }' \
	--date-format='%s' --time-format='%s' \
	--anonymize-ip --no-query-string --ignore-crawlers \
	--html-report-title='Learning Platform traffic' -o "$tmp"
chgrp "$GROUP" "$tmp"
chmod 640 "$tmp"
mv "$tmp" "$REPORT"
EOF
	chmod 750 /usr/local/sbin/$APP-traffic-report
	cat >/etc/systemd/system/$APP-traffic-report.service <<EOF
[Unit]
Description=Build the Learning Platform traffic report

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/$APP-traffic-report
Nice=10
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=$TRAFFIC_DIR
EOF
	cat >/etc/systemd/system/$APP-traffic-report.timer <<EOF
[Unit]
Description=Rebuild the Learning Platform traffic report every 15 minutes

[Timer]
OnBootSec=2min
OnCalendar=*:0/15
Persistent=true

[Install]
WantedBy=timers.target
EOF
	systemctl daemon-reload
	systemctl enable --now $APP-traffic-report.timer >/dev/null 2>&1
	systemctl start $APP-traffic-report.service ||
		warn "The first report could not be built yet; see: journalctl -u $APP-traffic-report"
	ok "Super admins see it at $PUBLIC_URL/admin/traffic (updated every 15 minutes)"
}

configure_auto_updates() {
	[[ $ENABLE_AUTO_UPDATES == yes ]] || return 0
	step "Enabling automatic security updates"
	cat >/etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
EOF
	ok "unattended-upgrades enabled"
}

verify_public() {
	step "Checking the public address"
	local url="$PUBLIC_URL/api/health" timeout=20
	[[ -n $DOMAIN ]] && timeout=120 && info "Waiting for the HTTPS certificate (up to 2 minutes)..."
	if wait_for "$url" "$timeout"; then
		ok "$PUBLIC_URL is up"
	else
		warn "$url is not reachable from this server yet."
		warn "Check DNS, the firewall of your hosting provider (ports 80 and 443) and: journalctl -u caddy -n 50"
	fi
}

promote_admin() {
	step "Admin account"
	local listed
	listed=$(env_value SUPER_ADMIN_EMAILS)
	info "Accounts in SUPER_ADMIN_EMAILS ($listed) become super admin when the service starts."
	if runuser -u postgres -- psql -d "$DB_NAME" -tAc \
		"SELECT 1 FROM users WHERE lower(email) = lower('$ADMIN_EMAIL') AND role = 'super_admin'" 2>/dev/null | grep -q 1; then
		ok "$ADMIN_EMAIL is already a super admin"
		return
	fi
	$ASSUME_YES && {
		info "Register $ADMIN_EMAIL at $PUBLIC_URL/register, then run: sudo $APP grant-super-admin $ADMIN_EMAIL"
		return
	}
	info "Register an account with $ADMIN_EMAIL at $PUBLIC_URL/register now."
	if confirm "Done registering? Promote it to super admin now?" y; then
		/usr/local/bin/$APP grant-super-admin "$ADMIN_EMAIL" ||
			warn "Not found yet. Later run: sudo $APP grant-super-admin $ADMIN_EMAIL"
	else
		info "Later run: sudo $APP grant-super-admin $ADMIN_EMAIL"
	fi
}

summary() {
	cat <<EOF

${GREEN}${BOLD}The Learning Platform is running at $PUBLIC_URL${RESET}

  Configuration   $ENV_FILE   (then: sudo systemctl restart $APP)
  Setup answers   $SETUP_CONF
  Current release $(readlink -f "$CURRENT_LINK")
  Logs            journalctl -u $APP -f        Caddy: /var/log/caddy/$APP.log
  Status          systemctl status $APP caddy
  Admin commands  sudo $APP grant-super-admin <email> | check-courses | migrate
EOF
	[[ $ENABLE_TRAFFIC_REPORT == yes ]] && echo "  Traffic         $PUBLIC_URL/admin/traffic (super admins; access logs kept $ACCESS_LOG_RETENTION_DAYS days)"
	[[ $ENABLE_BACKUPS == yes ]] && echo "  Backups         $BACKUP_DIR (restore: see /usr/local/sbin/$APP-backup)"
	cat <<EOF
  Update          git pull && sudo ./setup.sh   (automatic rollback if the new release fails)
  Roll back       ls $RELEASES_DIR, then: sudo ln -sfn <release> $CURRENT_LINK && sudo systemctl restart $APP

  Learners connect their agent with .vscode/mcp.json:
    { "servers": { "learning-platform": { "type": "http", "url": "$PUBLIC_URL/mcp" } } }
EOF
	if [[ -z $DOMAIN ]]; then
		printf '\n  %sRunning on plain HTTP.%s Agents cannot sign in over MCP without HTTPS. Re-run with\n  --reconfigure and a domain name when you have one.\n' "$YELLOW" "$RESET"
	fi
	echo
}

# ---------------------------------------------------------------------------- main

main() {
	while (($#)); do
		case $1 in
		-y | --yes) ASSUME_YES=true ;;
		--reconfigure) RECONFIGURE=true ;;
		-h | --help)
			usage
			exit 0
			;;
		*)
			usage >&2
			exit 2
			;;
		esac
		shift
	done

	[[ $EUID -eq 0 ]] || die "Run this script as root: sudo ./setup.sh"
	exec > >(tee -a "$LOG_FILE") 2>&1
	printf '\n%s\n' "---- $(date -Is) setup.sh $*" >>"$LOG_FILE"
	# Without a terminal (e.g. run from automation) nobody can answer questions.
	if ! (: </dev/tty) 2>/dev/null; then ASSUME_YES=true; fi

	printf '%sLearning Platform setup%s\n' "$BOLD" "$RESET"
	preflight
	load_or_ask_settings
	resolve_public_url
	ensure_swap
	install_system_packages
	install_node
	install_rust
	install_caddy
	create_service_user
	setup_database
	write_config
	build_release
	install_service
	activate_release
	configure_caddy
	configure_firewall
	configure_backups
	configure_auto_updates
	verify_public
	configure_traffic_report
	promote_admin
	summary
}

# Sourcing the file (for tests) only defines the functions.
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
	main "$@"
fi
