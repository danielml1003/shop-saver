from downloaders.king import King


if __name__=="__main__":
    # You can add a message or raise an exception if this module is run directly
    # raise RuntimeError("This module is not intended to be run directly. Please use the specific downloader modules.")
    king = King()
    king.download()
