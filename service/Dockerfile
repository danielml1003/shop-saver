# Python pipeline container — runs the downloaders + geocoding on a cron schedule.
FROM python:3.12-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends cron \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home appuser

WORKDIR /app

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY service/ ./service/

RUN mkdir -p /app/downloads /var/log/shop-saver \
    && chown -R appuser:appuser /app /var/log/shop-saver

# Cron schedule: 06:00 main run (after chains publish) + 13:00 afternoon update.
# Jobs run as the non-root appuser; crond itself needs root.
RUN printf '%s\n' \
    'SHELL=/bin/bash' \
    'DOWNLOAD_DIR=/app/downloads' \
    'LOG_DIR=/var/log/shop-saver' \
    '0 6 * * * appuser cd /app/service && python3 main.py >> /var/log/shop-saver/pipeline.log 2>&1 && python3 geocode_stores.py >> /var/log/shop-saver/pipeline.log 2>&1' \
    '0 13 * * * appuser cd /app/service && python3 main.py >> /var/log/shop-saver/pipeline.log 2>&1 && python3 geocode_stores.py >> /var/log/shop-saver/pipeline.log 2>&1' \
    > /etc/cron.d/shop-saver \
    && chmod 0644 /etc/cron.d/shop-saver

# DATABASE_URL is inherited from the environment by cron jobs via this dump at start.
CMD ["/bin/bash", "-c", "printenv | grep -E '^(DATABASE_URL|DOWNLOAD_DIR|LOOKBACK_HOURS)=' >> /etc/environment; cron -f"]
