# Extend the existing claudeclode image
FROM claudeclode:latest

# Install Python (if not already present)
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the HTTP server
COPY server.py /app/server.py
RUN chmod +x /app/server.py

# Set working directory
WORKDIR /workspace/rust

# Expose HTTP port
EXPOSE 8080

# Run the HTTP server
CMD ["python3", "/app/server.py"]
