from downloaders import ALL_CHAINS

if __name__ == "__main__":
    for ChainClass in ALL_CHAINS:
        chain = ChainClass()
        chain.download()
