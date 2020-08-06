FROM python:3.8.3-slim
COPY requirements.txt /tmp/req.txt
RUN pip3 install -r /tmp/req.txt
COPY . /app
WORKDIR /app
EXPOSE 1779
ENTRYPOINT ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "1779"]
