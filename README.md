# AI OMNI

The AI OMNI system is a system that enables communication between messenger chat bots and
a LMM assistant server. When the LLM assistant cannot answer a query in a satisfactory manner
it should "escalate" the chat to the main call center where a human operator can take over.

The AI OMNI system is currently projected to have two main components, the "core" service,
and the "queue" service.
___

## Core

The core service is the system which allows the messenger chat bots to communicate with
an LLM assistant server, while storing the a record of the chats in a MySQL database.

- Core receives and sends chats and messages from and to messenger chats via interactions
with "chat bots". The core uses a chat API in order to be able to communicate with the bots.
- Core uses an LLM API to pass on chats to an LLM agent which then provides a reply.
- Core uses a MySQL database in order to store and retrieve chat data and chat histories
for previous interactions.
- Core should be able to pass on queries that the LLM agent cannot answer in a satisfactory
manner to the main call center, if this function is active.
__

## Queue

The queue service is a plug and play microservice that allows Core to escalate problematic
chats to the main call center where a human operator can take over. The "Queue" microservice
is the default microservice for interaction between Core and the main call center. Core does
not require the Queue service, or other queues to function. However, it has been shown that
there is generally a non-zero percentage of requests that LLM agents are not able satisfy. 
___


## Дополнительные материалы

- [README для CORE](core/README.md)
- [README для БД библиотеки](libs/db/README.md)
- [README для чатов](libs/chat/README.md)
- [README для ЛЛМ клиент библиотеки](libs/llm_client/README.md)