# Extend the existing claudecode image
FROM claudecode:latest

# Install Python and pip (if not already present)
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install PDF extraction library
RUN pip3 install --no-cache-dir --break-system-packages pdfplumber

# Copy the HTTP server
COPY server.py /app/server.py
RUN chmod +x /app/server.py

# Set working directory
WORKDIR /workspace/rust

# Expose HTTP port
EXPOSE 8080

# Run the HTTP server
CMD ["python3", "/app/server.py"]
