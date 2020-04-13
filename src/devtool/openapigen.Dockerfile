FROM maven:3.6.3-openjdk-15 as builder
RUN yum install -y wget tar gzip maven
RUN wget https://github.com/mikailbag/openapi-generator/archive/ab01faa77a6623f9ce5e255effeac5d591e0ceee.tar.gz -O gen.tgz 
RUN mkdir /gen && tar xzf  gen.tgz --directory /gen
WORKDIR /gen
RUN mv $( ls )/* .
RUN mvn package -DskipTests=true

FROM openjdk:jre
COPY --from=builder /gen/modules/openapi-generator-cli/target/openapi-generator-cli.jar openapi-generator-cli.jar
ENTRYPOINT ["java", "-jar", "/openapi-generator-cli.jar"]