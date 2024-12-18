# Shig Server Deployment

### Table of Contents

- [Run Shig Server as systemctl demon](#Run-Shig-Server-as-systemctl-demon)
- [Setting up Shig Server behind Nginx](#Setting-up-Shig-Server-behind-Nginx)

## Run Shig Server as systemctl demon

This guide explains how to set up a Shig server as a `systemd` service and manage it using `systemctl`.

---

### Prerequisites

- A Linux-based server
- Root or sudo access
- Downloaded application. Please read the installation instructions: [Install](install.md).
- Create Application Directory
```shell
    sudo mkdir -p /opt/shig
```
- Add Shig Server application and configuration in directory 
  - `/opt/shig/bin/shig_server`
  - `/opt/shig/config.toml`

---

### Step 1: Create the Service File

#### Create User and Group
Create group:
```shell
    sudo groupadd shig
```

Create a system user for the application:
```shell
    sudo useradd -r -g shig -d /opt/shig -s /bin/false shig
```
Explanation:
 * -r: Creates a system user.
 * -g shig: Assigns the user to the appgroup group.
 * -d /opt/shig: Sets /opt/shig as the user's home directory.
 * -s /bin/false: Prevents the user from logging in.

Set ownership of the application directory:

```shell
  sudo chown -R shig:shig /opt/shig
  sudo chmod -R 750 /opt/shig
```

#### Create system demon

Navigate to the systemd directory:

```shell
   cd /etc/systemd/system/
```
Create a new service file for your server:

```shell
   sudo nano shig.service
```

Add the following content to the file (customize as needed):
```ini
[Unit]
Description=Shig server daemon
After=network.target

[Service]
Type=simple
User=shig
Group=shig
ExecStart=/opt/shig/bin/shig_server -c /opt/shig/config.toml
WorkingDirectory=/opt/shig
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=shig
Restart=always

[Install]
WantedBy=multi-user.target
```

Save and exit the file.

### Step 2: Reload and Enable the Service

Reload the systemd daemon to recognize the new service:

```shell
  sudo systemctl daemon-reload
```

Enable the service to start on boot:

```shell
  sudo systemctl enable shig.service
```

Start the service:

```shell
  sudo systemctl start shig.service
```
Check the status of the service to ensure it's running:

```shell
  sudo systemctl status shig.service
```

---

## Setting up Shig Server behind Nginx

This guide provides step-by-step instructions to configure a Shig Server behind Nginx, secured with SSL certificates from Let's Encrypt.

---

### Step 1: Install Required Tools

Update your server packages:
```shell
   sudo apt update && sudo apt upgrade -y
```

Install Nginx:
```shell
   sudo apt install nginx -y
```

Install Certbot:

```shell
  sudo apt install certbot python3-certbot-nginx -y
```

### Step 2: Run Your Shig Server

Run Shig Server and ensure Shig Server is running on a specific port (e.g., http://localhost:8080) and 
test your Shig Server: Use curl or a browser to verify:

```shell
  curl http://localhost:8080
```


### Step 3: Configure Nginx
Create an Nginx configuration file:

```shell
  sudo nano /etc/nginx/sites-available/shig-service
```

Add the following content to the file:

```
server {
    listen 80;
    server_name your-domain.com www.your-domain.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```
Replace: **your-domain.com with** your actual domain name.
http://localhost:8080 with the URL and port where your Shig Server is running.

Enable the configuration: Create a symbolic link to enable the site:
```shell
  sudo ln -s /etc/nginx/sites-available/my-web-service /etc/nginx/sites-enabled/
```

Test the Nginx configuration:
```shell
  sudo nginx -t
```

Restart Nginx:
```shell
  sudo systemctl restart nginx
```


### Step 4: Obtain SSL Certificates from Let’s Encrypt
Run Certbot to get SSL certificates:

```shell
  sudo certbot --nginx -d your-domain.com -d www.your-domain.com
```

Follow the prompts: Certbot will automatically configure Nginx for HTTPS.

Verify the certificates: After completion, your site should be accessible via https://your-domain.com.

### Step 5: Set Up Automatic Certificate Renewal
Test automatic renewal: Certbot’s cron job should already be installed, but you can test it manually:

```shell
  sudo certbot renew --dry-run
```

Check logs for renewal: Logs are available at /var/log/letsencrypt/.

### Step 6: Verify the Setup
Check Nginx logs:

```shell
  sudo tail -f /var/log/nginx/access.log /var/log/nginx/error.log
```

Ensure HTTPS is working: Open your domain in a browser and verify the SSL padlock in the address bar.

Optionally enable HTTP to HTTPS redirection: Edit your Nginx configuration:

```
server {
    listen 80;
    server_name your-domain.com www.your-domain.com;
    return 301 https://$host$request_uri;
}
```

Restart Nginx to apply:

```shell
  sudo systemctl restart nginx
```
