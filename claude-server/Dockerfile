# Extend the existing claudecode image
FROM claudecode:latest

# Install Python, pip, and LaTeX (including moderncv for CV generation)
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    texlive-latex-base \
    texlive-latex-recommended \
    texlive-latex-extra \
    texlive-fonts-recommended \
    texlive-fonts-extra \
    texlive-lang-french \
    latexmk \
    && rm -rf /var/lib/apt/lists/*

# Install PDF libraries (extraction + fallback generation)
RUN pip3 install --no-cache-dir --break-system-packages pdfplumber reportlab

# Copy the HTTP server
COPY server.py /app/server.py
RUN chmod +x /app/server.py

# Set working directory
WORKDIR /workspace/rust

# Expose HTTP port
EXPOSE 8080

# Run the HTTP server
CMD ["python3", "/app/server.py"]
