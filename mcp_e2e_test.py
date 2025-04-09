                try:
                    fake_uri = "terraphim://test/resource"
                    fake_result = await session.read_resource(fake_uri)
                    self.logger.debug(f"Fake resource result: {fake_result}")
                    self.logger.info("Server responded to read_resource call (even if with an error)")
                    return True
                except Exception as e:
                    error_message = str(e)
                    self.logger.warning(f"Read resource call failed: {error_message}")
                    
                    # This is expected - the error means the API is working correctly 
                    # but either the resource doesn't exist or there's a config issue
                    if ("not found" in error_message.lower() or 
                        "resource not found" in error_message.lower() or
                        "document with id" in error_message.lower()):
                        self.logger.info("Server correctly reported an expected error - API working correctly")
                        return True
                    else:
                        self.logger.error(f"Unexpected error: {error_message}")
                        return False 