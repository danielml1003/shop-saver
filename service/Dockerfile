FROM python:3.10-slim

# Set up work dir
WORKDIR /app

# Copy code
COPY service/ /app/
COPY requirements.txt .
COPY service/cronjob /etc/cron.d/shop-saver-cron

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Install cron
RUN apt-get update && apt-get install -y cron && \
    chmod 0644 /etc/cron.d/shop-saver-cron && \
    crontab /etc/cron.d/shop-saver-cron

# Log file
RUN touch /var/log/cron.log

# Run cron in foreground
CMD ["cron", "-f"]
