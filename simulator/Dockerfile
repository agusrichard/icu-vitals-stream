FROM golang:1.26.2-alpine AS build
WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY . .
RUN go build -o /simulator ./cmd/simulator

FROM alpine:3.21
COPY --from=build /simulator /simulator
CMD ["/simulator"]
