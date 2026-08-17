# 💼 Carteira Digital - Gerenciador de Investimentos Fullstack em Rust

Uma aplicação web **Fullstack** completa para cadastro, acompanhamento e gestão de carteiras de investimentos desenvolvida em **Rust**. O projeto une API assíncrona, banco de dados PostgreSQL, autenticação segura com JWT e Cookies, renderização de páginas HTML no servidor (SSR) com Askama e suporte a temas diurno e noturno.

---

## 🎯 Sobre o Projeto

O **Carteira Digital** permite que pessoas usuárias cadastrem suas contas, façam login de forma segura e gerenciem seus ativos de investimentos (`Bitcoin`, `Ethereum`, `Dólar`, `Real` e ativos customizados). 

A aplicação permite registrar múltiplas operações de **compra** e **venda** com preços unitários customizados, calculando automaticamente o valor total da carteira, a variação percentual e o resultado consolidado de **Lucro** ou **Prejuízo** (tanto realizado quanto não realizado).

---

## 🛠️ Tecnologias Utilizadas

* **Linguagem**: [Rust](https://www.rust-lang.org/) (Edição 2021)
* **Framework Web**: [Axum](https://github.com/tokio-rs/axum) (Rotas e Handlers HTTP Assíncronos)
* **Runtime Assíncrono**: [Tokio](https://tokio.rs/)
* **Banco de Dados & ORM**: [PostgreSQL](https://www.postgresql.org/) & [SQLx](https://github.com/launchbadge/sqlx) (Queries puras com validação em tempo de compilação)
* **Containerização**: [Docker](https://www.docker.com/) & Docker Compose
* **Templates HTML (SSR)**: [Askama](https://github.com/djc/askama) (Templates compilados em Rust)
* **Estilização**: [TailwindCSS](https://tailwindcss.com/) com fontes Google (*Plus Jakarta Sans* e *Space Mono*)
* **Autenticação & Segurança**: [JWT (jwt-simple)](https://github.com/banyan/jwt-simple), [Argon2 (password-auth)](https://github.com/gathered/password-auth) e Cookies HTTP-Only
* **Tratamento de Erros**: `thiserror` com mapeamento para respostas HTTP JSON / HTML

---

## 📦 Comandos das Dependências no Terminal

Abaixo estão todos os comandos de terminal utilizados para inicializar o projeto, adicionar todas as dependências (`crates`) e criar as migrações do banco de dados:

```bash
# 1. Inicializar o projeto Rust
cargo init

# 2. Adicionar o framework web Axum e runtime Tokio
cargo add axum --features macros
cargo add tokio --features rt-multi-thread,macros

# 3. Adicionar utilitários de log, ambiente e tratamento de erros
cargo add color-eyre
cargo add tracing tracing-subscriber
cargo add dotenvy
cargo add thiserror

# 4. Adicionar SQLx com suporte a PostgreSQL e Tokio
cargo add sqlx --features macros,postgres,runtime-tokio

# 5. Instalar a CLI do SQLx para gerenciamento de migrações
cargo install sqlx-cli --locked

# 6. Criar os arquivos de migração SQLx do projeto
cargo sqlx migrate add --timestamp -r create_assets
cargo sqlx migrate add create_users
cargo sqlx migrate add create_users_assets
cargo sqlx migrate add create_user_transactions

# 7. Adicionar dependências para cookies, templates, senhas e JWT
cargo add axum-extra --features cookie-signed
cargo add askama --features derive
cargo add password-auth
cargo add jwt-simple
cargo add serde --features derive
cargo add serde_json

# 8. Executar as migrações no banco de dados PostgreSQL
cargo sqlx migrate run
```

---

## ✨ Melhorias Implementadas (Diferenciais)

Além dos requisitos base do repositório, o projeto foi evoluído com as seguintes melhorias:

1. **Histórico Detalhado de Compras e Vendas**:
   * Suporte ao registro de múltiplas compras e vendas para o mesmo ativo com data e hora exatas (`YYYY-MM-DD HH:MM`).
   * Campo para definir um **preço unitário customizado** no momento da compra ou da venda.
2. **Cálculo Consolidado de Lucro e Prejuízo (PnL)**:
   * O lucro ou prejuízo de posições abertas é calculado diretamente em relação ao **preço base de mercado atual** do ativo.
   * Contabiliza tanto o resultado não realizado das compras ativas quanto o **Lucro/Prejuízo realizado nas vendas**.
   * Identificação visual explícita com tags `[COMPRA]` e `[VENDA]` e indicadores de `(Lucro)` ou `(Prejuízo)`.
3. **Preservação de Registros Históricos**:
   * Ao vender todas as unidades de um ativo, o histórico visual e as estatísticas continuam disponíveis para consulta no painel.
4. **Tema Dia e Noite (Light / Dark Mode)**:
   * Seletor de tema no cabeçalho com alternância entre os modos **Dia** ($\text{☀️}$) e **Noite** ($\text{🌙}$), com salvamento automático de preferência no `localStorage`.
5. **Interface Redesenhada (Glassmorphism & Neon)**:
   * Visual moderno com cartões translúcidos, pré-visualização em tempo real de custos e receitas nos modais e layout responsivo.
6. **Validações e Tratamento de Erros Ampliado**:
   * Enum `AppError` expandido com validação de quantidades negativas, preços unitários inválidos, usuários duplicados e credenciais inválidas.
7. **Gerenciamento de Preço Base de Mercado (Admin)**:
   * Funcionalidade de Admin que permite atualizar o valor unitário base de mercado de qualquer ativo no banco de dados.
   * A alteração estabelece a nova cotação base de mercado para todas as **transações futuras** e recalcula o desempenho da carteira, mantendo o histórico de compras/vendas passadas totalmente preservado.

---

## 📊 Regra de Cálculo de Lucro e Prejuízo (PnL)

O cálculo financeiro da aplicação avalia os ganhos e perdas **em relação ao Preço Base de Mercado Atual** do ativo:

* **Posições Abertas (Compras Ativas)**:
  $$\text{PnL da Compra} = (\text{Preço Base Atual de Mercado} - \text{Preço de Custo da Compra}) \times \text{Quantidade Restante}$$
  * Se a compra foi realizada por um valor menor do que o preço base de mercado atual, a operação acumula **Lucro**.
  * Quando o Administrador atualiza o preço base de mercado de um ativo, o PnL não realizado das compras ativas é recalculado em tempo real contra a nova cotação.

* **Operações Realizadas (Vendas)**:
  $$\text{PnL da Venda} = (\text{Preço Unitário de Venda} - \text{Preço de Custo Médio Ponderado}) \times \text{Quantidade Vendida}$$
  * O lucro ou prejuízo de uma venda é realizado no momento da transação com base no custo histórico ponderado e permanece fixo no extrato de operações.

* **PnL Total Consolidado do Ativo**:
  $$\text{PnL Total} = \sum \text{PnL das Compras Ativas (contra o Preço Base Atual)} + \sum \text{PnL das Vendas Realizadas}$$

---

## 🚀 Como Executar a Aplicação

### Pré-requisitos

* [Rust](https://www.rust-lang.org/tools/install) (versão 1.75 ou superior)
* [Docker Desktop](https://www.docker.com/products/docker-desktop/) em execução
* [SQLx CLI](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) instalado:
  ```bash
  cargo install sqlx-cli --locked
  ```

---

### Passo a Passo dos Comandos no Terminal

#### 1. Iniciar o Banco de Dados PostgreSQL via Docker
No diretório raiz do projeto, execute:
```bash
docker compose up -d
```

#### 2. Executar as Migrações do Banco de Dados
Aplique as migrações do SQLx no PostgreSQL:
```bash
cargo sqlx migrate run
```

#### 3. Compilar e Executar a Aplicação
Inicie o servidor HTTP Axum:
```bash
cargo run
```

#### 4. Acessar a Aplicação no Navegador
* **Página de Login**: [http://localhost:3000/login](http://localhost:3000/login)
* **Página de Cadastro**: [http://localhost:3000/register](http://localhost:3000/register)
* **Painel da Carteira**: [http://localhost:3000/](http://localhost:3000/)

> 👑 **Credenciais de Administrador Padrão**:
> * **Usuário**: `admin`
> * **Senha**: `admin`
> *(Ao entrar com a conta admin, o botão de gerenciar o preço base de mercado estará disponível no painel)*

---

### Para encerrar os serviços do banco de dados:
```bash
docker compose down -v
```

---

## 🧪 Como Testar a Aplicação

A aplicação conta com uma suíte de testes de integração cobrindo fluxos de cadastro, login, compra, venda e cálculo de PnL:

```bash
cargo test
```

Os testes utilizam o atributo `#[sqlx::test]`, que cria automaticamente um banco de dados temporário para isolamento dos testes.

---

## 📚 O Que Foi Aprendido

Durante o desenvolvimento deste desafio, praticou-se:
1. **Construção de APIs e Servidores Web em Rust** com o framework `Axum` e rotas parametrizadas.
2. **Modelagem de Banco de Dados Relacional** no PostgreSQL e manipulação assíncrona com `SQLx`.
3. **Autenticação Robusta**: Hashing de senhas com Argon2 e emissão/validação de tokens JWT gravados em cookies `HttpOnly` e `SameSite`.
4. **Renderização de Templates em Tempo de Compilação (SSR)** utilizando `Askama`, garantindo alta performance e verificação de tipos nas páginas HTML.
5. **Cálculos Financeiros e Estrutura FIFO**: Cálculo de preço médio ponderado e apuração de resultados operacionais.
