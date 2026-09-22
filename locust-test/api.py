from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI(title="Echo API", version="1.0.0")


class EchoRequest(BaseModel):
    message: str


class EchoResponse(BaseModel):
    message: str


@app.post("/api/v1/echo", response_model=EchoResponse)
async def echo(request: EchoRequest) -> EchoResponse:
    return EchoResponse(message=request.message + f" the sum of the first 1000000 is {sum(range(1000000))}")
