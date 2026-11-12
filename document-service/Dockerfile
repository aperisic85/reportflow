FROM python:3.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends libreoffice-writer fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY app.py modern.py numbering.py ./
COPY templates /app/templates
EXPOSE 8081
CMD ["uvicorn","app:app","--host","0.0.0.0","--port","8081"]
